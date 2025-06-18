extern crate proc_macro;

mod macros;
mod utils;
use once_cell::sync::OnceCell;
use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::{parse_macro_input, punctuated::Punctuated, ExprAssign, Token};
use utils::types::*;

static CONFIG: OnceCell<CommonConfig> = OnceCell::new();
static TYPES: OnceCell<TypesConfig> = OnceCell::new();

#[proc_macro]
pub fn include_operon(input: TokenStream) -> TokenStream {

    let operon = macros::write_operon_module(
        TYPES.get().expect("Types not initialized"),
        CONFIG.get().expect("Config not initialized"),
    );
    let operon_internal = quote! {};
    quote! {
        #operon
        #operon_internal
    }
    .into()
}
