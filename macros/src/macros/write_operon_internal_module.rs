use proc_macro2::TokenStream;
use quote::quote;
use crate::{TypesConfig, CommonConfig};

pub fn write_operon_internal_module(_entities: &TypesConfig, _config: &CommonConfig) -> TokenStream {
    quote! {
        pub mod operon_internal {
            use super::operon::*;
        }
    }
}
