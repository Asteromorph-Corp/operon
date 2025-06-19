extern crate proc_macro;

mod macros;
mod utils;
use once_cell::sync::OnceCell;
use proc_macro::TokenStream;
use quote::quote;
use utils::{config_types::*, parse::*};

static CONFIG: OnceCell<AllConfig> = OnceCell::new();

#[proc_macro]
pub fn include_operon(input: TokenStream) -> TokenStream {
    let maybe_config = get_config(input);
    let config = match maybe_config {
        Ok(config) => config,
        Err(e) => return e.to_compile_error().into(),
    };
    CONFIG.set(config)
        .expect("Config already initialized");
    let operon = macros::write_operon_module(
        CONFIG.get().expect("Config not initialized"),
    );
    let operon_internal = macros::write_operon_internal_module(
        CONFIG.get().expect("Config not initialized"),
    );
    quote! {
        #operon
        #operon_internal
    }
    .into()
}
