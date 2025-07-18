mod configs;
mod macros;
mod utils;
use crate::{
    configs::{AllConfig, DimensionConfig, EntityConfig, JobArg, JobConfig, JobConfigMap},
    macros::operon,
};
use proc_macro::TokenStream;
use quote::quote;

#[proc_macro]
pub fn define_operon(input: TokenStream) -> TokenStream {
    let all_configs: AllConfig = match syn::parse(input) {
        Ok(config) => config,
        Err(e) => return e.to_compile_error().into(),
    };
    let operon = operon(&all_configs);

    quote! {
        #operon
    }
    .into()
}
