use proc_macro2::TokenStream;
use quote::quote;
use crate::AllConfig;

pub fn write_operon_internal_module(_config: &AllConfig) -> TokenStream {
    quote! {
        pub mod operon_internal {
            use super::operon::*;
        }
    }
}
