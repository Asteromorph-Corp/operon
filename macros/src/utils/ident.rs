use heck::{ToPascalCase, ToShoutySnakeCase, ToSnakeCase};
use proc_macro_crate::{FoundCrate, crate_name};
use quote::format_ident;
use syn::parse_quote;

use crate::configs::{DimensionId, JobId};

pub fn operon_ident() -> syn::Ident {
    match crate_name("operon") {
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

pub fn get_entity_ident(entity_id: &str) -> syn::Ident {
    format_ident!("get_{}", entity_id.to_snake_case())
}

pub fn put_entity_ident(entity_id: &str) -> syn::Ident {
    format_ident!("put_{}", entity_id.to_snake_case())
}

pub fn init_resolution_ident(dimension_id: &DimensionId) -> syn::Ident {
    format_ident!("init_resolution_{}", dimension_id.to_snake_case())
}

pub fn clear_resolution_ident(dimension_id: &DimensionId) -> syn::Ident {
    format_ident!("clear_resolution_{}", dimension_id.to_snake_case())
}

pub fn get_resolution_ident(dimension_id: &DimensionId) -> syn::Ident {
    format_ident!("get_resolution_{}", dimension_id.to_snake_case())
}

pub fn put_resolution_ident(dimension_id: &DimensionId) -> syn::Ident {
    format_ident!("put_resolution_{}", dimension_id.to_snake_case())
}

pub fn init_ticket_ident(job_id: &JobId) -> syn::Ident {
    format_ident!("init_ticket_{}", job_id.to_snake_case())
}

pub fn clear_ticket_ident(job_id: &JobId) -> syn::Ident {
    format_ident!("clear_ticket_{}", job_id.to_snake_case())
}

pub fn put_ticket_ident(job_id: &JobId) -> syn::Ident {
    format_ident!("put_ticket_{}", job_id.to_snake_case())
}

pub fn get_all_ident(job_id: &JobId) -> syn::Ident {
    format_ident!("get_all_{}", job_id.to_snake_case())
}

pub fn mark_done_ident(job_id: &JobId) -> syn::Ident {
    format_ident!("mark_done_{}", job_id.to_snake_case())
}

pub fn explode_ident(job_id: &JobId, dimension_id: &DimensionId) -> syn::Ident {
    format_ident!(
        "explode_{}_{}",
        job_id.to_snake_case(),
        dimension_id.to_snake_case()
    )
}

pub fn raise_dep_ident(job_id: &JobId) -> syn::Ident {
    format_ident!("raise_dep_{}", job_id.to_snake_case())
}

pub fn dimension_ident(dimension_id: &DimensionId) -> syn::Ident {
    format_ident!("{}Dim", dimension_id.to_pascal_case())
}

pub fn variant_ident(id: &str) -> syn::Ident {
    format_ident!("{}", id.to_pascal_case())
}

pub fn variable_ident(id: &str) -> syn::Ident {
    format_ident!("{}", id.to_snake_case())
}

pub fn sender_ident(job_id: &JobId) -> syn::Ident {
    format_ident!("to_{}", job_id.to_snake_case())
}

pub fn with_ident(dimension_id: &DimensionId) -> syn::Ident {
    format_ident!("with_{}", dimension_id.to_snake_case())
}

pub fn job_ident(job_id: &JobId) -> syn::Ident {
    format_ident!("{}Job", job_id.to_pascal_case())
}

pub fn resolution_ident(dimension_id: &DimensionId) -> syn::Ident {
    format_ident!("{}Resolution", dimension_id.to_pascal_case())
}

pub fn ticket_ident(job_id: &JobId) -> syn::Ident {
    format_ident!("{}Ticket", job_id.to_pascal_case())
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

pub fn spawn_resolution(spawn_dim: Option<&DimensionId>) -> syn::Type {
    match spawn_dim {
        Some(dim) => {
            let res_ident = resolution_ident(dim);
            parse_quote! { schema::#res_ident }
        }
        None => parse_quote! { () },
    }
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
