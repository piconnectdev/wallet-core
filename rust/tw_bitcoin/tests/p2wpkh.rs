use std::str::FromStr;

use bitcoin::{PubkeyHash, PublicKey, Txid, WPubkeyHash};
use tw_bitcoin::{
    keypair_from_wif, Recipient, TransactionBuilder, TxInputP2PKH, TxInputP2WPKH, TxOutputP2WPKH,
};
use tw_encoding::hex;

mod common;
use common::*;

pub const ALICE_WIF: &str = "cQX5ePcXjTx7C5p6xV8zkp2NN9unhZx4a8RQVPiHd52WxoApV6yK";
pub const BOB_WIF: &str = "cMn7SSCtE5yt2PS97P4NCMvxpCVvT4cBuHiCzKFW5XMvio4fQbD1";
pub const GENESIS_BLOCK: &str = "6fed131159018b4caea7af1ffe1c747d6405cf55cccfe69d1909a79f672200e5";
pub const GENESIS_TXID: &str = "181c84965c9ea86a5fac32fdbd5f73a21a7a9e749fb6ab97e273af2329f6b911";

#[test]
fn sign_input_p2pkh_and_p2wpkh_output_p2wpkh() {
    // This passed the `bitcoin-cli -retest testmempoolaccept` command.
    const EXPECTED_RAW_SIGNED: &str = "020000000111b9f62923af73e297abb69f749e7a1aa2735fbdfd32ac5f6aa89e5c96841c18000000006b483045022100df9ed0b662b759e68b89a42e7144cddf787782a7129d4df05642dd825930e6e6022051a08f577f11cc7390684bbad2951a6374072253ffcf2468d14035ed0d8cd6490121028d7dce6d72fb8f7af9566616c6436349c67ad379f2404dd66fe7085fe0fba28fffffffff01c0aff629010000001600140d0e1cec6c2babe8badde5e9b3dea667da90036d00000000";

    const COINBASE_AMOUNT: u64 = ONE_BTC * 50;
    const MINER_FEE: u64 = ONE_BTC / 100;
    const SEND_TO_BOB: u64 = COINBASE_AMOUNT - MINER_FEE;

    let alice = keypair_from_wif(ALICE_WIF).unwrap();
    let bob = keypair_from_wif(BOB_WIF).unwrap();

    // # First transaction: Alice spends the P2PKH coinbase input and creates
    // # a P2WPKH output for Bob.

    // Prepare inputs for Alice.
    let txid = Txid::from_str(GENESIS_TXID).unwrap();
    let vout = 0;
    let recipient = Recipient::<PubkeyHash>::from_keypair(&alice);
    let satoshis = COINBASE_AMOUNT;

    let input = TxInputP2PKH::new(txid, vout, recipient, Some(satoshis));

    // Prepare outputs for Bob.
    let recipient = Recipient::<WPubkeyHash>::from_keypair(&bob);
    let satoshis = SEND_TO_BOB;

    let output = TxOutputP2WPKH::new(satoshis, recipient);

    // Alice signs the transaction.
    let signed_transaction = TransactionBuilder::new(bitcoin::Network::Regtest)
        .miner_fee(MINER_FEE)
        .add_input(input.into())
        .add_output(output.into())
        .sign_inputs(alice)
        .unwrap()
        .serialize()
        .unwrap();

    let hex = hex::encode(signed_transaction, false);
    assert_eq!(&hex, EXPECTED_RAW_SIGNED);

    // # Second transaction: Bob spends the P2WPKH input and creates
    // # a P2WPKH output for Alice.

    // Transaction was submitted in regtest env via `sendrawtransaction` and
    // mined with `-generate 1`
    const EXPECTED_RAW_SIGNED_SECOND: &str = "020000000001016e1f16dcfafbb3a83697f6c23c624cd71085a7f8a25ce0bd9743a41d0a458e850000000000ffffffff01806de7290100000016001460cda7b50f14c152d7401c28ae773c698db9237302483045022100a9b517de5a5e036d7133df499b5b751db6f9a01576a6c5dc38229ec08b6c45cd02200e42c9f8c707c9bf0ceab4f739ec8d683dc1f1f29e195a8da9bc183584d624a60121025a0af1510f0f24d40dd00d7c0e51605ca504bbc177c3e19b065f373a1efdd22f00000000";
    const LATEST_TXID: &str = "858e450a1da44397bde05ca2f8a78510d74c623cc2f69736a8b3fbfadc161f6e";
    const SEND_TO_ALICE: u64 = SEND_TO_BOB - MINER_FEE;

    let txid = Txid::from_str(LATEST_TXID).unwrap();
    let vout = 0;
    let recipient = Recipient::<WPubkeyHash>::from_keypair(&bob);
    let satoshis = SEND_TO_BOB;

    let input = TxInputP2WPKH::new(txid, vout, recipient, Some(satoshis));

    // Prepare outputs for Bob.
    let recipient = Recipient::<WPubkeyHash>::from_keypair(&alice);
    let satoshis = SEND_TO_ALICE;

    let output = TxOutputP2WPKH::new(satoshis, recipient);

    // Alice signs the transaction.
    let signed_transaction = TransactionBuilder::new(bitcoin::Network::Regtest)
        .miner_fee(MINER_FEE)
        .add_input(input.into())
        .add_output(output.into())
        .sign_inputs(bob)
        .unwrap()
        .serialize()
        .unwrap();

    let hex = hex::encode(signed_transaction, false);
    assert_eq!(&hex, EXPECTED_RAW_SIGNED_SECOND);
}
