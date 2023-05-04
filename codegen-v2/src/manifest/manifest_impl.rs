use crate::grammar::{
    GEnumDecl, GFunctionDecl, GHeaderInclude, GMarker, GMarkers, GPrimitive, GStructDecl, GType,
    GTypeCategory, GTypedef,
};
use crate::manifest::*;
use heck::ToLowerCamelCase;

// NOTE: This function is temporary
pub fn process_c_grammar(dir: &CHeaderDirectory) -> Vec<FileInfo> {
    let mut file_infos = vec![];

    for (path, items) in &dir.map {
        let file_name = path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .strip_suffix(".h")
            .unwrap()
            .to_string();

        let mut file_info = FileInfo {
            name: file_name.clone(),
            structs: vec![],
            inits: vec![],
            deinits: vec![],
            enums: vec![],
            functions: vec![],
            properties: vec![],
            protos: vec![],
        };

        for item in items {
            match item {
                GHeaderFileItem::Typedef(decl) => {
                    if decl.name.contains("Proto") {
                        let x = ProtoInfo::from_g_type(decl).unwrap();
                        file_info.protos.push(x);
                    }
                }
                GHeaderFileItem::StructIndicator(decl) => {
                    let markers = &decl.markers.0;

                    let mut tags = vec![];
                    match markers.first() {
                        Some(GMarker::TwExportStruct) => {
                            tags.push("TW_EXPORT_STRUCT".to_string());
                        }
                        Some(GMarker::TwExportClass) => {
                            tags.push("TW_EXPORT_CLASS".to_string());
                        }
                        _ => {}
                    };

                    file_info.structs.push(StructInfo {
                        name: decl.name.0 .0.clone(),
                        is_public: true,
                        fields: vec![],
                        tags,
                    });
                }
                GHeaderFileItem::StructDecl(decl) => {
                    let x = StructInfo::from_g_type(decl).unwrap();
                    file_info.structs.push(x);
                }
                GHeaderFileItem::EnumDecl(decl) => {
                    let x = EnumInfo::from_g_type(decl).unwrap();
                    file_info.enums.push(x);
                }
                GHeaderFileItem::FunctionDecl(decl) => {
                    if decl.name.0.starts_with("TWAnySigner") {
                        println!(
                            "Skipped function from manifest (non-export):  {}",
                            decl.name.0
                        );
                        continue;
                    }

                    let markers = &decl.markers.0;

                    // Handle exported properties.
                    if markers.contains(&GMarker::TwExportProperty)
                        || markers.contains(&GMarker::TwExportStaticProperty)
                    {
                        let x = PropertyInfo::from_g_type(decl).unwrap();
                        file_info.properties.push(x);
                    }
                    // Handle methods
                    else {
                        // Detect constructor methods.
                        if decl.name.0.contains("Create") {
                            let x = InitInfo::from_g_type(decl).unwrap();
                            file_info.inits.push(x);
                        }
                        // Delect deconstructor methods.
                        else if decl.name.0.contains("Delete") {
                            let x = DeinitInfo::from_g_type(decl).unwrap();
                            file_info.deinits.push(x);
                        }
                        // Any any other method is just a method.
                        else {
                            let x = FunctionInfo::from_g_type(&None, decl).unwrap();
                            file_info.functions.push(x);
                        }
                    }
                }
                _ => {}
            }
        }

        file_infos.push(file_info);
    }

    file_infos
}

impl TypeInfo {
    pub fn from_g_type(ty: &GType, markers: &GMarkers) -> Result<Self> {
        pub fn get_variant(category: &GTypeCategory) -> Result<TypeVariant> {
            let variant = match category {
                GTypeCategory::Scalar(s) => match s {
                    GPrimitive::Void => TypeVariant::Void,
                    GPrimitive::Bool => TypeVariant::Bool,
                    GPrimitive::Char => TypeVariant::Char,
                    GPrimitive::ShortInt => TypeVariant::ShortInt,
                    GPrimitive::Int => TypeVariant::Int,
                    GPrimitive::UnsignedInt => TypeVariant::UnsignedInt,
                    GPrimitive::LongInt => TypeVariant::LongInt,
                    GPrimitive::Float => TypeVariant::Float,
                    GPrimitive::Double => TypeVariant::Double,
                    GPrimitive::SizeT => TypeVariant::SizeT,
                    GPrimitive::Int8T => TypeVariant::Int8T,
                    GPrimitive::Int16T => TypeVariant::Int16T,
                    GPrimitive::Int32T => TypeVariant::Int32T,
                    GPrimitive::Int64T => TypeVariant::Int64T,
                    GPrimitive::UInt8T => TypeVariant::UInt8T,
                    GPrimitive::UInt16T => TypeVariant::UInt16T,
                    GPrimitive::UInt32T => TypeVariant::UInt32T,
                    GPrimitive::UInt64T => TypeVariant::UInt64T,
                },
                GTypeCategory::Struct(s) => TypeVariant::Struct(s.0 .0.to_string()),
                GTypeCategory::Enum(e) => TypeVariant::Enum(e.0 .0.to_string()),
                GTypeCategory::Pointer(_) | GTypeCategory::Unrecognized(_) => {
                    return Err(Error::BadType);
                }
            };

            Ok(variant)
        }

        pub fn get_variant_pointer_check(cat: &GTypeCategory) -> Result<(TypeVariant, bool)> {
            let res = match cat {
                GTypeCategory::Pointer(p) => (get_variant(&*p)?, true),
                _ => (get_variant(cat)?, false),
            };

            Ok(res)
        }

        let is_nullable = markers.0.iter().any(|m| match m {
            GMarker::Nullable => true,
            GMarker::NonNull => false,
            _ => false,
        });

        // Special handler for `TW_DATA` and `TW_STRING`.
        match ty {
            GType::Mutable(GTypeCategory::Pointer(pointer))
            | GType::Const(GTypeCategory::Pointer(pointer)) => {
                if let GTypeCategory::Unrecognized(ref keyword) = **pointer {
                    if keyword.0 == "TWData" {
                        return Ok(TypeInfo {
                            variant: TypeVariant::Data,
                            // Is always const
                            is_constant: true,
                            is_nullable,
                            is_pointer: true,
                            tags: vec!["TW_DATA".to_string()],
                        });
                    } else if keyword.0 == "TWString" {
                        return Ok(TypeInfo {
                            variant: TypeVariant::String,
                            // Is always const
                            is_constant: true,
                            is_nullable,
                            is_pointer: true,
                            tags: vec!["TW_STRING".to_string()],
                        });
                    }
                }
            }
            _ => {}
        }

        let ((variant, is_pointer), is_constant) = match ty {
            GType::Mutable(category) => (get_variant_pointer_check(category)?, false),
            GType::Const(category) => (get_variant_pointer_check(category)?, true),
            GType::Extern(_) => {
                return Err(Error::BadType);
                // TODO: Where should this be handled?
            }
        };

        Ok(TypeInfo {
            variant,
            is_constant,
            is_nullable,
            is_pointer,
            tags: vec![],
        })
    }
}

impl ImportInfo {
    pub fn from_g_type(value: &GHeaderInclude) -> Result<Self> {
        let mut path: Vec<String> = value.0.split('/').map(|s| s.to_string()).collect();

        if path.is_empty() {
            return Err(Error::BadImport);
        }

        if let Some(file_name) = path.last_mut() {
            *file_name = file_name
                .strip_suffix(".h")
                .ok_or(Error::BadImport)
                .map(|s| s.to_string())?;
        }

        Ok(ImportInfo { path })
    }
}

impl ProtoInfo {
    pub fn from_g_type(value: &GTypedef) -> Result<Self> {
        let mut found = false;
        match value.ty {
            GType::Mutable(ref cat) | GType::Const(ref cat) => {
                if let GTypeCategory::Pointer(p) = cat {
                    if let GTypeCategory::Unrecognized(ref keyword) = **p {
                        if keyword.0 == "TWData" {
                            found = true;
                        }
                    }
                }
            }
            _ => return Err(Error::BadObject),
        }

        if !found {
            return Err(Error::BadObject);
        }

        Ok(ProtoInfo(value.name.clone()))
    }
}

impl EnumInfo {
    pub fn from_g_type(value: &GEnumDecl) -> Result<Self> {
        // Read the docs of the custom function for more info.
        if value.name.0 == "TWStellarPassphrase" {
            return Ok(super::manifest_impl_custom::custom_handle_stellar_passphrase());
        } else if value.name.0 == "TWHRP" {
            return Ok(super::manifest_impl_custom::custom_handle_hrp());
        }

        if value.markers.0.len() != 1 {
            return Err(Error::BadObject);
        }

        let marker = value.markers.0.first().unwrap();
        let value_ty = if let GMarker::TwExportEnum(ty) = marker {
            TypeInfo::from_g_type(ty, &GMarkers(vec![]))?.variant
        } else {
            return Err(Error::BadObject);
        };

        Ok(EnumInfo {
            name: value.name.0.to_string(),
            // Enums are always public
            // TOOD: Should be part of GEnumDecl
            is_public: true,
            value_type: value_ty,
            variants: value
                .variants
                .iter()
                .cloned()
                .enumerate()
                .map(|(idx, (k, v))| {
                    let mut name =
                        k.0.strip_prefix(&value.name.0)
                            .unwrap()
                            .to_lower_camel_case();

                    // HACK
                    // Some variant names do not follow standard camelCase convention.
                    if value.name.0 == "TWCoinType" {
                        name = name.replace("xDai", "xdai");
                        name = name.replace("polygonzkEvm", "polygonzkEVM");
                        name = name.replace("ecoChain", "ecochain");
                        name = name.replace("okxChain", "okxchain");
                        name = name.replace("ioTeXevm", "ioTeXEVM");
                        name = name.replace("poaNetwork", "poanetwork");
                        name = name.replace("thorChain", "thorchain");
                        name = name.replace("eCash", "ecash");
                        name = name.replace("fetchAi", "fetchAI");
                    }

                    EnumVariantInfo {
                        // Remove prefix from enum variant.
                        name,
                        // In the old codegen, non-values result in a simple
                        // counter. IMO fixed values should be enforced.
                        value: v.unwrap_or(idx),
                        as_string: None,
                    }
                })
                .collect(),
            tags: vec![],
        })
    }
}

impl StructInfo {
    pub fn from_g_type(value: &GStructDecl) -> Result<Self> {
        let mut markers = value.markers.0.iter();

        if markers.size_hint().0 != 1 {
            return Err(Error::BadObject);
        }

        // Identify whether it's a struct or a class. The object must be *one*
        // of the two available markers and is always public
        let mut tags = vec![];
        match markers.next() {
            Some(GMarker::TwExportStruct) => {
                tags.push("TW_EXPORT_STRUCT".to_string());
            }
            Some(GMarker::TwExportClass) => {
                tags.push("TW_EXPORT_CLASS".to_string());
            }
            _ => return Err(Error::BadObject),
        };

        // Process fields.
        let fields = value
            .fields
            .iter()
            .map(|(k, v)| {
                Ok((
                    k.0.to_string(),
                    // In this context, types inside a C struct do not support
                    // markers.
                    TypeInfo::from_g_type(v, &GMarkers(vec![]))?,
                ))
            })
            .collect::<Result<Vec<(String, TypeInfo)>>>()?;

        Ok(StructInfo {
            name: value.name.0 .0.to_string(),
            is_public: true,
            fields,
            tags,
        })
    }
}

impl PropertyInfo {
    pub fn from_g_type(value: &GFunctionDecl) -> Result<Self> {
        // ### Name

        // Strip the object name from the property name.
        // E.g. "SomeObjectIsValid" => "IsValid"
        let name = value.name.0.clone();

        if name.is_empty() {
            return Err(Error::BadProperty);
        }

        // ### Marker

        let mut markers = value.markers.0.iter();

        // Must have one marker.
        if markers.size_hint().0 != 1 {
            return Err(Error::BadProperty);
        }

        // The property must have one of the two available markers and is always public.
        let (_is_static, is_public) = match markers.next() {
            Some(GMarker::TwExportProperty) => (false, true),
            Some(GMarker::TwExportStaticProperty) => (true, true),
            _ => return Err(Error::BadObject),
        };

        // ### Param

        // Must have at least one parameter.
        if value.params.len() < 1 {
            return Err(Error::BadProperty);
        }

        // ### Return value

        // Extract return value.
        let re = &value.return_value;
        let return_type = TypeInfo::from_g_type(&re.ty, &re.markers)?;

        Ok(PropertyInfo {
            name,
            is_public,
            return_type,
            comments: vec![],
        })
    }
}

impl InitInfo {
    pub fn from_g_type(value: &GFunctionDecl) -> Result<Self> {
        let func = FunctionInfo::from_g_type(&None, value)?;

        let is_public = value
            .markers
            .0
            .iter()
            .any(|m| m == &GMarker::TwExportMethod || m == &GMarker::TwExportStaticMethod);

        let is_nullable = value.return_value.markers.0.iter().any(|m| match m {
            GMarker::Nullable => true,
            GMarker::NonNull => false,
            _ => false,
        });

        Ok(InitInfo {
            name: func.name,
            is_public,
            is_nullable,
            params: func.params,
            comments: vec![],
        })
    }
}

impl DeinitInfo {
    pub fn from_g_type(value: &GFunctionDecl) -> Result<Self> {
        let func = FunctionInfo::from_g_type(&None, value)?;

        Ok(DeinitInfo { name: func.name })
    }
}

impl FunctionInfo {
    pub fn from_g_type(object_name: &Option<String>, value: &GFunctionDecl) -> Result<Self> {
        // ### Name

        // Strip the object name from the method name.
        // E.g. "SomeObjectIsValid" => "IsValid"
        let name = if let Some(object_name) = object_name {
            value
                .name
                .0
                .strip_prefix(object_name)
                .ok_or(Error::BadType)?
                .to_string()
        } else {
            value.name.0.to_string()
        };

        if name.is_empty() {
            return Err(Error::BadType);
        }

        // ### Marker

        let mut markers = value.markers.0.iter();

        // The method must have one of the two available markers and is always public.
        let (is_static, is_public) = match markers.next() {
            Some(GMarker::TwExportMethod) => (false, true),
            Some(GMarker::TwExportStaticMethod) => (true, true),
            // TODO:?
            //_ => return Err(Error::BadObject),
            _ => (false, false),
        };

        // ### Params

        let mut g_params = value.params.iter();

        // Convert parameters.
        let mut params = vec![];
        while let Some(g_item) = g_params.next() {
            params.push(ParamInfo {
                name: g_item.name.0.to_string(),
                ty: TypeInfo::from_g_type(&g_item.ty, &g_item.markers)?,
            })
        }

        // ### Return value

        // Extract return value.
        let re = &value.return_value;
        let return_type = TypeInfo::from_g_type(&re.ty, &re.markers)?;

        Ok(FunctionInfo {
            name,
            is_public,
            is_static,
            params,
            return_type,
            comments: vec![],
        })
    }
}