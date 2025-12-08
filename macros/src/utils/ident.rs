use heck::{ToPascalCase, ToShoutySnakeCase, ToSnakeCase};
use once_cell::sync::Lazy;
use proc_macro_crate::{FoundCrate, crate_name};
use quote::format_ident;

static CRATE_NAME: Lazy<Result<FoundCrate, proc_macro_crate::Error>> =
    Lazy::new(|| crate_name("operon"));

pub fn operon_ident() -> syn::Ident {
    match CRATE_NAME.as_ref() {
        Ok(FoundCrate::Name(name)) => format_ident!("{name}"),
        Ok(FoundCrate::Itself) => format_ident!("crate"),
        Err(_) => format_ident!("operon"),
    }
}

pub fn service_trait_ident(service_id: &str) -> syn::Ident {
    format_ident!("{}Service", service_id.to_pascal_case())
}

pub fn storage_trait_ident(service_id: &str) -> syn::Ident {
    format_ident!("{}Storage", service_id.to_pascal_case())
}

pub fn entities_ident(service_id: &str) -> syn::Ident {
    format_ident!("{}Entities", service_id.to_pascal_case())
}

pub fn sql_storage_ident(service_id: &str) -> syn::Ident {
    format_ident!("Psql{}Storage", service_id.to_pascal_case())
}

pub fn get_handler_ident(service_id: &str) -> syn::Ident {
    format_ident!("{}_handler", service_id.to_snake_case())
}

pub fn job_metadata_ident(job_id: &syn::Ident) -> syn::Ident {
    let name = format!("job_{}_meta", job_id.to_string().to_snake_case());
    syn::Ident::new(&name, job_id.span())
}

pub fn dimension_metadata_ident(dimension_id: &syn::Ident) -> syn::Ident {
    let name = format!(
        "dimension_{}_meta",
        dimension_id.to_string().to_snake_case()
    );
    syn::Ident::new(&name, dimension_id.span())
}

pub fn entity_metadata_ident(entity_id: &syn::Ident) -> syn::Ident {
    let name = format!("entity_{}_meta", entity_id.to_string().to_snake_case());
    syn::Ident::new(&name, entity_id.span())
}

pub fn get_entity_ident(entity_id: &syn::Ident) -> syn::Ident {
    let name = format!("get_{}", entity_id.to_string().to_snake_case());
    syn::Ident::new(&name, entity_id.span())
}

pub fn put_entity_ident(entity_id: &syn::Ident) -> syn::Ident {
    let name = format!("put_{}", entity_id.to_string().to_snake_case());
    syn::Ident::new(&name, entity_id.span())
}

pub fn batch_get_entity_ident(entity_id: &syn::Ident, over: &[syn::Ident]) -> syn::Ident {
    let name = format!(
        "get_all_{}_{}",
        entity_id.to_string().to_snake_case(),
        over.iter()
            .map(|d| d.to_string().to_snake_case())
            .collect::<String>()
    );
    syn::Ident::new(&name, entity_id.span())
}

pub fn batch_put_entity_ident(entity_id: &syn::Ident) -> syn::Ident {
    let name = format!("put_all_{}", entity_id.to_string().to_snake_case());
    syn::Ident::new(&name, entity_id.span())
}

pub fn entity_over_dim_ident(entity_id: &syn::Ident, over: &[syn::Ident]) -> syn::Ident {
    let name = if over.is_empty() {
        entity_id.to_string().to_snake_case()
    } else {
        format!(
            "{}_{}",
            entity_id.to_string().to_snake_case(),
            over.iter()
                .map(|d| d.to_string().to_snake_case())
                .collect::<String>()
        )
    };
    syn::Ident::new(&name, entity_id.span())
}

pub fn sender_ident(job_id: &syn::Ident) -> syn::Ident {
    let name = format!("to_{}", job_id.to_string().to_snake_case());
    syn::Ident::new(&name, job_id.span())
}

pub fn spec_ident(job_id: &syn::Ident) -> syn::Ident {
    let name = format!("{}Spec", job_id.to_string().to_pascal_case());
    syn::Ident::new(&name, job_id.span())
}

pub fn rebuilder_ident(job_id: &syn::Ident) -> syn::Ident {
    let name = format!("{}Rebuilder", job_id.to_string().to_pascal_case());
    syn::Ident::new(&name, job_id.span())
}

pub fn peer_txs_ident(job_id: &syn::Ident) -> syn::Ident {
    let name = format!("{}PeerTxs", job_id.to_string().to_pascal_case());
    syn::Ident::new(&name, job_id.span())
}

pub fn resolution_enum_ident() -> syn::Ident {
    format_ident!("ResolutionEnum")
}

pub fn job_enum_ident() -> syn::Ident {
    format_ident!("JobEnum")
}

pub fn job_id_ident(job_id: &syn::Ident) -> syn::Ident {
    let name = format!("{}_ID", job_id.to_string().to_shouty_snake_case());
    syn::Ident::new(&name, job_id.span())
}

pub fn to_pascal_case(id: &syn::Ident) -> syn::Ident {
    syn::Ident::new(&id.to_string().to_pascal_case(), id.span())
}

pub fn to_snake_case(id: &syn::Ident) -> syn::Ident {
    syn::Ident::new(&id.to_string().to_snake_case(), id.span())
}

pub fn to_lit_str(ident: &syn::Ident) -> syn::LitStr {
    syn::LitStr::new(&ident.to_string().to_snake_case(), ident.span())
}

pub fn to_type(ident: &syn::Ident) -> syn::Type {
    syn::Type::Path(syn::TypePath {
        qself: None,
        path: syn::Path::from(syn::PathSegment::from(ident.clone())),
    })
}
