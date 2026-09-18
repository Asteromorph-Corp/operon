use heck::{ToPascalCase, ToShoutySnakeCase, ToSnakeCase};
use once_cell::sync::Lazy;
use proc_macro_crate::{FoundCrate, crate_name};
use quote::format_ident;

static CRATE_NAME: Lazy<Result<FoundCrate, proc_macro_crate::Error>> =
    Lazy::new(|| crate_name("operon"));

pub(crate) fn operon_ident() -> syn::Ident {
    match CRATE_NAME.as_ref() {
        Ok(FoundCrate::Name(name)) => format_ident!("{name}"),
        Ok(FoundCrate::Itself) => format_ident!("operon"),
        Err(_) => format_ident!("operon"),
    }
}

pub(crate) fn service_trait_ident_spanned(service_id: &syn::Ident) -> syn::Ident {
    let name = format!("{}Service", service_id.to_string().to_pascal_case());
    syn::Ident::new(&name, service_id.span())
}

pub(crate) fn service_trait_ident(service_id: &syn::Ident) -> syn::Ident {
    format_ident!("{}Service", service_id.to_string().to_pascal_case())
}

pub(crate) fn storage_trait_ident(service_id: &syn::Ident) -> syn::Ident {
    format_ident!("{}Storage", service_id.to_string().to_pascal_case())
}

pub(crate) fn entities_ident(service_id: &syn::Ident) -> syn::Ident {
    format_ident!("{}Entities", service_id.to_string().to_pascal_case())
}

pub(crate) fn sql_storage_ident(service_id: &syn::Ident) -> syn::Ident {
    format_ident!("Psql{}Storage", service_id.to_string().to_pascal_case())
}

pub(crate) fn mem_storage_ident(service_id: &syn::Ident) -> syn::Ident {
    format_ident!("Mem{}Storage", &service_id.to_string().to_pascal_case())
}

pub(crate) fn mem_storage_options_ext_ident(service_id: &syn::Ident) -> syn::Ident {
    format_ident!(
        "Mem{}StorageOptionsExt",
        &service_id.to_string().to_pascal_case()
    )
}

pub(crate) fn task_metadata_ident(task_id: &syn::Ident) -> syn::Ident {
    format_ident!("task_{}_meta", task_id.to_string().to_snake_case())
}

pub(crate) fn dimension_metadata_ident(dimension_id: &syn::Ident) -> syn::Ident {
    format_ident!(
        "dimension_{}_meta",
        dimension_id.to_string().to_snake_case()
    )
}

pub(crate) fn entity_metadata_ident(entity_id: &syn::Ident) -> syn::Ident {
    format_ident!("entity_{}_meta", entity_id.to_string().to_snake_case())
}

pub(crate) fn get_entity_ident(entity_id: &syn::Ident) -> syn::Ident {
    format_ident!("get_{}", entity_id.to_string().to_snake_case())
}

pub(crate) fn put_entity_ident(entity_id: &syn::Ident) -> syn::Ident {
    format_ident!("put_{}", entity_id.to_string().to_snake_case())
}

pub(crate) fn batch_get_entity_ident(entity_id: &syn::Ident, over: &[syn::Ident]) -> syn::Ident {
    format_ident!(
        "get_all_{}_{}",
        entity_id.to_string().to_snake_case(),
        over.iter()
            .map(|d| d.to_string().to_snake_case())
            .collect::<String>()
    )
}

pub(crate) fn batch_put_entity_ident(entity_id: &syn::Ident) -> syn::Ident {
    format_ident!("put_all_{}", entity_id.to_string().to_snake_case())
}

pub(crate) fn entity_over_dim_ident(entity_id: &syn::Ident, over: &[syn::Ident]) -> syn::Ident {
    if over.is_empty() {
        return format_ident!("{}", entity_id.to_string().to_snake_case());
    }
    format_ident!(
        "{}_{}",
        entity_id.to_string().to_snake_case(),
        over.iter()
            .map(|d| d.to_string().to_snake_case())
            .collect::<String>()
    )
}

pub(crate) fn sender_ident(task_id: &syn::Ident) -> syn::Ident {
    format_ident!("to_{}", task_id.to_string().to_snake_case())
}

pub(crate) fn spec_ident(task_id: &syn::Ident) -> syn::Ident {
    format_ident!("{}Spec", task_id.to_string().to_pascal_case())
}

pub(crate) fn rebuilder_ident(task_id: &syn::Ident) -> syn::Ident {
    format_ident!("{}Rebuilder", task_id.to_string().to_pascal_case())
}

pub(crate) fn peer_txs_ident(task_id: &syn::Ident) -> syn::Ident {
    format_ident!("{}PeerTxs", task_id.to_string().to_pascal_case())
}

pub(crate) fn resolution_enum_ident() -> syn::Ident {
    format_ident!("ResolutionEnum")
}

pub(crate) fn job_enum_ident() -> syn::Ident {
    format_ident!("JobEnum")
}

pub(crate) fn ticket_enum_ident() -> syn::Ident {
    format_ident!("TicketEnum")
}

pub(crate) fn task_id_ident(task_id: &syn::Ident) -> syn::Ident {
    format_ident!("{}_ID", task_id.to_string().to_shouty_snake_case())
}

pub(crate) fn to_pascal_case(id: &syn::Ident) -> syn::Ident {
    format_ident!("{}", id.to_string().to_pascal_case())
}

pub(crate) fn to_snake_case(id: &syn::Ident) -> syn::Ident {
    format_ident!("{}", id.to_string().to_snake_case())
}

pub(crate) fn clear_span(ident: &syn::Ident) -> syn::Ident {
    syn::Ident::new(&ident.to_string(), proc_macro2::Span::call_site())
}

pub(crate) fn to_lit_str(ident: &syn::Ident) -> syn::LitStr {
    syn::LitStr::new(
        &ident.to_string().to_snake_case(),
        proc_macro2::Span::call_site(),
    )
}

pub(crate) fn to_type(ident: &syn::Ident) -> syn::Type {
    syn::Type::Path(syn::TypePath {
        qself: None,
        path: syn::Path::from(syn::PathSegment::from(ident.clone())),
    })
}
