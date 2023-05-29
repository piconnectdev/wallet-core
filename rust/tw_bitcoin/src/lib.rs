use bitcoin::blockdata::locktime::absolute::{Height, LockTime};
use bitcoin::blockdata::script::ScriptBuf;
use bitcoin::blockdata::transaction::OutPoint;
use bitcoin::hash_types::Txid;
use bitcoin::hashes::sha256d::Hash;
use bitcoin::key::UntweakedPublicKey;
use bitcoin::psbt::Input;
use bitcoin::sighash::{LegacySighash, SighashCache};
use bitcoin::taproot::{LeafVersion, TaprootBuilder};
use bitcoin::transaction::Transaction;
use bitcoin::{Sequence, TxIn, TxOut, Witness};
//use secp256k1::{generate_keypair, KeyPair, Secp256k1};
use tw_hash::H256;
use tw_keypair::ecdsa::secp256k1;
use tw_keypair::traits::{KeyPairTrait, SigningKeyTrait};

pub type Result<T> = std::result::Result<T, Error>;

const SIGHASH_ALL: u32 = 1;

fn convert_legacy_btc_hash_to_h256(hash: LegacySighash) -> H256 {
    let slice: &[u8] = hash.as_raw_hash().as_ref();
    debug_assert_eq!(slice.len(), 32);
    let bytes: [u8; 32] = slice.try_into().unwrap();
    H256::from(bytes)
}

// TODO: `tw_keypair` should offer this function/method.
fn to_der_signature(sig: &[u8; 64]) -> Vec<u8> {
    let mut der_sig = vec![];

    // Add the DER prefix
    der_sig.push(0x30);

    // Length of the rest of the signature
    der_sig.push(68);

    // Add the 'r' component
    der_sig.push(0x02); // integer marker
    der_sig.push(32); // length of the 'r' component
    der_sig.extend(&sig[0..32]);

    // Add the 's' component
    der_sig.push(0x02); // integer marker
    der_sig.push(32); // length of the 's' component
    der_sig.extend(&sig[32..64]);

    der_sig
}

#[derive(Debug, Clone)]
pub enum Error {
    Todo,
}

/*
#[test]
fn poc() {
    let secp = Secp256k1::new();
    let (private, public) = generate_keypair(&mut rand::thread_rng());
    let tweaked = UntweakedPublicKey::from(public);

    let script1 = ScriptBuf::new();
    let script2 = ScriptBuf::new();

    let spend_info = TaprootBuilder::new()
        .add_leaf(1, script1.clone())
        .unwrap()
        .add_leaf(1, script2)
        .unwrap()
        .finalize(&secp, tweaked)
        .unwrap();

    let root = spend_info.merkle_root().unwrap();

    let tapscript = ScriptBuf::new_v1_p2tr(&secp, tweaked, Some(root));

    let control_block = spend_info.control_block(&(script1, LeafVersion::TapScript));
}
*/

pub enum KeyPair {
    Secp256k1(secp256k1::KeyPair),
}

impl KeyPair {
    // TODO: Prefix `as_`
    pub fn private(&self) -> PrivateKeyRef {
        match self {
            KeyPair::Secp256k1(pair) => PrivateKeyRef::Secp256k1(pair.private()),
        }
    }
    pub fn public(&self) -> PublicKeyRef {
        match self {
            KeyPair::Secp256k1(pair) => PublicKeyRef::Secp256k1(pair.public()),
        }
    }
}

pub enum PrivateKeyRef<'a> {
    Secp256k1(&'a secp256k1::PrivateKey),
}

impl<'a> PrivateKeyRef<'a> {
    fn sign_h256(&self, msg: H256) -> Result<Signature> {
        match self {
            PrivateKeyRef::Secp256k1(key) => Ok(Signature::Secp256k1(
                key.sign(msg).map_err(|_| Error::Todo)?,
            )),
        }
    }
}

pub enum PublicKey {
    Secp256k1(secp256k1::PublicKey),
}

pub enum PublicKeyRef<'a> {
    Secp256k1(&'a secp256k1::PublicKey),
}

impl<'a> PublicKeyRef<'a> {
    pub fn hash(&self) -> PublicKeyHash {
        todo!()
    }
}

pub enum Signature {
    Secp256k1(secp256k1::Signature),
}

impl Signature {
    pub fn to_der(&self) -> &[u8] {
        match self {
            Signature::Secp256k1(sig) => sig.to_der(),
        }
    }
}

pub struct PublicKeyHash(bitcoin::PubkeyHash);
pub struct ScriptHash;

pub struct TransactionBuilder {
    version: i32,
    lock_time: LockTime,
    keypair: KeyPair,
    inputs: Vec<TxInput>,
    outputs: Vec<TxOutput>,
}

impl TransactionBuilder {
    fn new(keypair: KeyPair) -> Self {
        TransactionBuilder {
            // TODO: Check this.
            version: 2,
            // No lock time, transaction is immediately spendable.
            lock_time: LockTime::Blocks(Height::ZERO),
            keypair,
            inputs: vec![],
            outputs: vec![],
        }
    }
    pub fn version(mut self, version: i32) -> Self {
        self.version = version;
        self
    }
    // TODO: handle locktime seconds?.
    pub fn lock_time(mut self, height: u32) -> Self {
        self.lock_time = LockTime::Blocks(Height::from_consensus(height).unwrap());
        self
    }
    fn add_input(mut self, input: TxInput) -> Self {
        self.inputs.push(input);
        self
    }
    fn add_input_from_utxo(mut self, utxo: TxOut, point: OutPoint) -> Self {
        match TxInputBuilder::from_utxo(utxo, point) {
            TxInputBuilder::P2pkh(builder) => {
                let input = builder
                    .my_pubkey_hash(self.keypair.public().hash())
                    .build()
                    // Panicing implies bug.
                    .unwrap();

                self.inputs.push(input);
            },
            TxInputBuilder::NonStandard => {
                todo!()
            },
        }

        self
    }
    /// Alias for `add_output_pk2pkh`.
    fn add_output_transfer(self, recipient: Recipient, satoshis: u64) -> Self {
        self.add_output_p2pkh(recipient, satoshis)
    }
    fn add_output_p2pkh(mut self, recipient: Recipient, satoshis: u64) -> Self {
        match recipient {
            Recipient::LegacyHash(hash) => {
                self.outputs.push(TxOutput::P2pkh {
                    satoshis,
                    script_pubkey: ScriptBuf::new_p2pkh(&hash.0),
                });
            },
            _ => todo!(),
        }

        self
    }
    fn build(self) -> Result<usize> {
        let mut tx = Transaction {
            version: self.version,
            lock_time: self.lock_time,
            input: vec![],
            output: vec![],
        };

        for input in &self.inputs {
            let btc_txin = match input {
                // TODO: `TxIn` should implement `From<TxInput>`.
                TxInput::P2pkh { ctx, hash: _ } => TxIn::from(ctx.clone()),
            };

            tx.input.push(btc_txin);
        }

        for output in self.outputs {
            let btc_txout = TxOut::from(output);
            tx.output.push(btc_txout);
        }

        let cache = SighashCache::new(tx);

        //let mut input_scriptsigs = vec![];

        for (index, input) in self.inputs.into_iter().enumerate() {
            // TODO: Prettify this.
            let script_pubkey = match input {
                TxInput::P2pkh { ctx, hash: _ } => ctx.script_pub_key,
            };

            let btc_hash = cache
                .legacy_signature_hash(
                    index,
                    // TODO: Add note that this is same as `scriptPubKey` handled
                    // somewhere else.
                    &script_pubkey,
                    SIGHASH_ALL,
                )
                .map_err(|_| Error::Todo)?;

            let hash = convert_legacy_btc_hash_to_h256(btc_hash);
            let sig = self.keypair.private().sign_h256(hash)?.to_der();

            //input_scriptsigs.push()
        }

        todo!()
    }
}

pub enum Recipient {
    LegacyHash(PublicKeyHash),
    LegacyPubkey(PublicKey),
    Segwit(()),
    Taproot(()),
}

#[derive(Debug, Clone)]
// TODO: Should be private.
pub struct InputContext {
    previous_output: OutPoint,
    // Inputs for `script_pub_key`.
    script_sig: ScriptBuf,
    // The condition for claiming the output.
    script_pub_key: ScriptBuf,
    // TODO: Document this.
    sequence: Sequence,
    // Witness data for Segwit/Taproot transactions.
    witness: Witness,
}

impl From<InputContext> for TxIn {
    fn from(ctx: InputContext) -> Self {
        TxIn {
            previous_output: ctx.previous_output,
            script_sig: ctx.script_sig,
            sequence: ctx.sequence,
            witness: ctx.witness,
        }
    }
}

impl InputContext {
    pub fn new(utxo: TxOut, point: OutPoint) -> Self {
        InputContext {
            previous_output: point,
            // Empty scriptbuf.
            script_sig: ScriptBuf::new(),
            // TODO: Document this.
            script_pub_key: utxo.script_pubkey,
            // Default value of `0xFFFFFFFF`.
            sequence: Sequence::default(),
            // Empty witness.
            witness: Witness::new(),
        }
    }
}

pub struct P2pkhInputBuilder {
    ctx: InputContext,
    hash: Option<PublicKeyHash>,
}

impl P2pkhInputBuilder {
    pub fn my_pubkey_hash(mut self, hash: PublicKeyHash) -> Self {
        self.hash = Some(hash);
        self
    }
    pub fn build(self) -> Result<TxInput> {
        Ok(TxInput::P2pkh {
            ctx: self.ctx,
            hash: self.hash.ok_or(Error::Todo)?,
        })
    }
}

pub enum TxOutput {
    P2pkh {
        satoshis: u64,
        script_pubkey: ScriptBuf,
    },
}

impl From<TxOutput> for TxOut {
    fn from(out: TxOutput) -> Self {
        match out {
            TxOutput::P2pkh {
                satoshis,
                script_pubkey,
            } => TxOut {
                value: satoshis,
                script_pubkey,
            },
        }
    }
}

pub enum TxInput {
    P2pkh {
        ctx: InputContext,
        hash: PublicKeyHash,
    },
}

pub enum TxInputBuilder {
    P2pkh(P2pkhInputBuilder),
    NonStandard,
}

impl TxInputBuilder {
    fn from_utxo(utxo: TxOut, point: OutPoint) -> Self {
        if utxo.script_pubkey.is_p2pkh() {
            TxInputBuilder::P2pkh(P2pkhInputBuilder {
                ctx: InputContext::new(utxo, point),
                hash: None,
            })
        } else {
            TxInputBuilder::NonStandard
        }
    }
}
