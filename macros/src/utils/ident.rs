use heck::{ToPascalCase, ToShoutySnakeCase, ToSnakeCase};
use once_cell::sync::Lazy;
use proc_macro_crate::{FoundCrate, crate_name};
use quote::format_ident;

use crate::configs::{EntityId, JobId};

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

pub fn job_metadata_ident(job_id: &JobId) -> syn::Ident {
    format_ident!("job_{}_meta", job_id.to_snake_case())
}

pub fn dimension_metadata_ident(dimension_id: &syn::Ident) -> syn::Ident {
    let name = format!(
        "dimension_{}_meta",
        dimension_id.to_string().to_snake_case()
    );
    syn::Ident::new(&name, dimension_id.span())
}

pub fn entity_metadata_ident(entity_id: &JobId) -> syn::Ident {
    format_ident!("entity_{}_meta", entity_id.to_snake_case())
}

pub fn get_entity_ident(entity_id: &EntityId) -> syn::Ident {
    format_ident!("get_{}", entity_id.to_snake_case())
}

pub fn put_entity_ident(entity_id: &EntityId) -> syn::Ident {
    format_ident!("put_{}", entity_id.to_snake_case())
}

pub fn batch_get_entity_ident(entity_id: &EntityId, over: &[syn::Ident]) -> syn::Ident {
    format_ident!(
        "get_all_{}_over_{}",
        entity_id.to_snake_case(),
        over.iter()
            .map(|d| d.to_string().to_snake_case())
            .collect::<String>()
    )
}

pub fn batch_put_entity_ident(entity_id: &EntityId) -> syn::Ident {
    format_ident!("put_all_{}", entity_id.to_snake_case())
}

pub fn job_fn_ident(job_id: &JobId) -> syn::Ident {
    format_ident!("{}", job_id.to_snake_case())
}

pub fn entity_over_dim_ident(entity_id: &EntityId, over: &[syn::Ident]) -> syn::Ident {
    format_ident!(
        "{}{}",
        entity_id.to_snake_case(),
        over.iter()
            .map(|d| format!("_{}", d.to_string().to_snake_case()))
            .collect::<String>()
    )
}

pub fn variant_ident(id: &syn::Ident) -> syn::Ident {
    syn::Ident::new(&id.to_string().to_pascal_case(), id.span())
}

pub fn variable_ident(id: &str) -> syn::Ident {
    format_ident!("{}", id.to_snake_case())
}

pub fn sender_ident(job_id: &JobId) -> syn::Ident {
    format_ident!("to_{}", job_id.to_snake_case())
}

pub fn entity_ident(entity_id: &EntityId) -> syn::Ident {
    format_ident!("{}", entity_id.to_pascal_case())
}

pub fn spec_ident(job_id: &JobId) -> syn::Ident {
    format_ident!("{}Spec", job_id.to_pascal_case())
}

pub fn rebuilder_ident(job_id: &JobId) -> syn::Ident {
    format_ident!("{}Rebuilder", job_id.to_pascal_case())
}

pub fn peer_txs_ident(job_id: &JobId) -> syn::Ident {
    format_ident!("{}PeerTxs", job_id.to_pascal_case())
}

pub fn resolution_enum_ident() -> syn::Ident {
    format_ident!("ResolutionEnum")
}

pub fn job_enum_ident() -> syn::Ident {
    format_ident!("JobEnum")
}

pub fn job_id_ident(job_id: &JobId) -> syn::Ident {
    format_ident!("{}_ID", job_id.to_shouty_snake_case())
}

pub fn as_lit_str(ident: &syn::Ident) -> syn::LitStr {
    syn::LitStr::new(&ident.to_string(), ident.span())
}
