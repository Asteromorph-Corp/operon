extern crate proc_macro;

mod macros;

mod configs;

mod utils;
use proc_macro::TokenStream;
use quote::quote;
use utils::{config_types::*, parse::*};

#[proc_macro]
pub fn include_operon(input: TokenStream) -> TokenStream {
    let maybe_config = get_config(input);
    let config = match maybe_config {
        Ok(config) => config,
        Err(e) => return e.to_compile_error().into(),
    };
    let operon = macros::write_operon_module(&config);
    let operon_internal = macros::write_operon_internal_module(&config);
    quote! {
        #operon
        #operon_internal
    }
    .into()
}
