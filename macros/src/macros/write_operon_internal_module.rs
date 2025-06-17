use proc_macro2::TokenStream;
use quote::quote;
use crate::{Entities, GlobalConfig};

pub fn write_operon_internal_module(_entities: &Entities, _config: &GlobalConfig) -> TokenStream {
    quote! {
        pub mod operon_internal {
            use super::operon::*;
        }
    }
}
