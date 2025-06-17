extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;

#[proc_macro]
pub fn include_operon(_input: TokenStream) -> TokenStream {
    quote! {

    }.into()
}

#[proc_macro]
pub fn use_psql_storage(_input: TokenStream) -> TokenStream {
    quote! {
        
    }.into()
}
