extern crate proc_macro;

mod macros;
mod utils;
use once_cell::sync::Lazy;
use utils::types::*;
use proc_macro::TokenStream;
use quote::quote;

static ENTITIES: Lazy<Entities> = Lazy::new(utils::parse::get_entities);
static CONFIG: Lazy<GlobalConfig> = Lazy::new(utils::parse::get_config);

#[proc_macro]
pub fn include_operon(_input: TokenStream) -> TokenStream {
    let operon = macros::write_operon_module(&ENTITIES, &CONFIG);
    let operon_internal = macros::write_operon_internal_module(&ENTITIES, &CONFIG);
    quote! {
        #operon
        #operon_internal
    }.into()
}

#[proc_macro]
pub fn use_psql_storage(_input: TokenStream) -> TokenStream {
    quote! {
        // TODO
    }.into()
}
