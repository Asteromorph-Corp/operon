use heck::ToPascalCase;
use proc_macro_crate::{FoundCrate, crate_name};
use quote::format_ident;

pub fn operon_ident() -> syn::Ident {
    match crate_name("operon") {
        Ok(FoundCrate::Name(name)) => format_ident!("{name}"),
        Ok(FoundCrate::Itself) => format_ident!("crate"),
        Err(_) => format_ident!("operon"),
    }
}

pub fn init_resolution_ident(dimension_id: &str) -> syn::Ident {
    format_ident!("init_resolution_{dimension_id}")
}

pub fn clear_resolution_ident(dimension_id: &str) -> syn::Ident {
    format_ident!("clear_resolution_{dimension_id}")
}

pub fn get_resolution_ident(dimension_id: &str) -> syn::Ident {
    format_ident!("get_resolution_{dimension_id}")
}

pub fn put_resolution_ident(dimension_id: &str) -> syn::Ident {
    format_ident!("put_resolution_{dimension_id}")
}

pub fn init_ticket_ident(job_id: &str) -> syn::Ident {
    format_ident!("init_ticket_{job_id}")
}

pub fn clear_ticket_ident(job_id: &str) -> syn::Ident {
    format_ident!("clear_ticket_{job_id}")
}

pub fn put_default_tickets_ident(job_id: &str) -> syn::Ident {
    format_ident!("put_default_tickets_{job_id}")
}

pub fn dimension_ident(dimension_id: &str) -> syn::Ident {
    format_ident!("{}", dimension_id.to_pascal_case())
}

pub fn resolution_ident(dimension_id: &str) -> syn::Ident {
    format_ident!("{}Resolution", dimension_id.to_pascal_case())
}

pub fn ticket_ident(job_id: &str) -> syn::Ident {
    format_ident!("{}Ticket", job_id.to_pascal_case())
}
