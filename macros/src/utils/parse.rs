use super::types::*;
use indexmap::IndexMap;
use quote::ToTokens;
use syn::{
    braced, bracketed, parse::{Parse, ParseStream}, punctuated::Punctuated, ExprAssign, Token
};

#[derive(Debug, Default)]
struct RawEntity {
    primary: Option<bool>,
    dims: Vec<String>,
    def: Option<String>,
    from: Option<Vec<String>>,
    pool: Option<usize>,
    body: String,
}

#[derive(Debug, Default)]
struct RawTypesConfig {
    entity: IndexMap<String, RawEntity>,
}
impl Parse for RawTypesConfig {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut types = RawTypesConfig::default();
        while !input.is_empty() {
            let _ = input.parse::<Token![#]>()?;
            let attr_content;
            let _ = bracketed!(attr_content in input);
            let attr_type = attr_content.parse::<syn::Ident>()?;
            if attr_type != "entity" {
                return Err(syn::Error::new_spanned(attr_type, "Expected `entity` attribute"));
            }
            while !attr_content.is_empty() {
                let arg_name = attr_content.parse::<syn::Ident>()?;
                let _ = attr_content.parse::<Token![=]>()?;
                match arg_name.to_string().as_str() {
                    _ => todo!()
                }
            }


            let _ = input.parse::<Token![;]>()?;
        }
        Ok(types)
    }
}

#[derive(Debug, Default)]
struct RawLogConfig {
    level: Option<String>,
    buffer_size: Option<usize>,
    dump: Option<bool>,
    dump_path: Option<String>,
}

#[derive(Debug, Default)]
struct RawCommonConfig {
    storage: StorageConfig,
    log: RawLogConfig,
}
impl Parse for RawCommonConfig {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut config = RawCommonConfig::default();
        while !input.is_empty() {
            let arg: ExprAssign = input.parse()?;
            match arg.left.to_token_stream().to_string().as_str() {
                "storage.data.uri" => {
                    config.storage.data.uri = arg.right.to_token_stream().to_string();
                }
                "storage.data.schema" => {
                    config.storage.data.schema = Some(arg.right.to_token_stream().to_string());
                }
                "storage.data.pool_size" => {
                    config.storage.data.pool_size =
                        Some(arg.right.to_token_stream().to_string().parse().unwrap());
                }
                "storage.metadata.uri" => {
                    config.storage.metadata.uri = arg.right.to_token_stream().to_string();
                }
                "storage.metadata.schema" => {
                    config.storage.metadata.schema = Some(arg.right.to_token_stream().to_string());
                }
                "storage.metadata.pool_size" => {
                    config.storage.metadata.pool_size =
                        Some(arg.right.to_token_stream().to_string().parse().unwrap());
                }
                "log.level" => {
                    config.log.level = Some(arg.right.to_token_stream().to_string());
                }
                "log.buffer_size" => {
                    config.log.buffer_size =
                        Some(arg.right.to_token_stream().to_string().parse().unwrap());
                }
                "log.dump" => {
                    config.log.dump =
                        Some(arg.right.to_token_stream().to_string().parse().unwrap());
                }
                "log.dump_path" => {
                    config.log.dump_path = Some(arg.right.to_token_stream().to_string());
                }
                _ => return Err(syn::Error::new_spanned(arg, "Unknown configuration option")),
            }
            let _ = input.parse::<Token![,]>();
        }
        Ok(config)
    }
}

#[derive(Debug, Default)]
struct RawAllConfig {
    types: RawTypesConfig,
    global: RawCommonConfig,
    use_psql_storage: Option<String>,
}
impl Parse for RawAllConfig {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut all_configs = RawAllConfig::default();
        while !input.is_empty() {
            let ident = input.parse::<syn::Ident>()?;
            match ident.to_string().as_str() {
                "config" => {
                    let content;
                    let _ = braced!(content in input);
                    all_configs.global = content.parse()?;
                }
                "types" => {
                    let content;
                    let _ = braced!(content in input);
                    all_configs.types = content.parse()?;
                }
                "use_psql_storage" => {
                    all_configs.use_psql_storage = Some(input.parse::<syn::Ident>()?.to_string());
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        ident,
                        "Unknown configuration section",
                    ));
                }
            }
            let _ = input.parse::<Token![;]>();
        }
        Ok(all_configs)
    }
}

fn resolve_types(raw: RawTypesConfig) -> TypesConfig {
    let mut entities: Entities = Vec::new();
    let mut dimensions: Dimensions = Vec::new();
    let mut jobs: Jobs = Vec::new();

    for (i, (entity_name, raw_entity)) in raw.entity.iter().enumerate() {
        let primary = match raw_entity.primary {
            Some(true) if i == 0 => true,
            Some(true) => {
                panic!(
                    "Only the first entity can be primary, but {entity_name} was marked as primary"
                )
            }
            Some(false) if i == 0 => {
                panic!(
                    "The first entity must be primary, but {entity_name} was marked as non-primary"
                )
            }
            Some(false) => false,
            None => i == 0,
        };
    }

    TypesConfig {
        entities,
        dimensions,
        jobs,
    }
}
