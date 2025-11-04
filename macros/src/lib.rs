mod configs;
mod dependency_analysis;
mod macros;
mod utils;

#[cfg(test)]
mod test_utils;

use proc_macro::TokenStream;
use proc_macro_error::ResultExt;
use quote::{ToTokens, quote};
use syn::{DeriveInput, parse_macro_input, parse_quote};

use crate::macros::operon;
use crate::utils::{extract_attr, get_operon_attrs, operon_ident};

// TODO: use text fixtures instead of constructing the configs in code.

#[proc_macro]
pub fn define_operon(input: TokenStream) -> TokenStream {
    let all_configs: crate::configs::AllConfig = match syn::parse(input) {
        Ok(config) => config,
        Err(e) => return e.to_compile_error().into(),
    };
    let operon = operon(&all_configs);
    operon.into_token_stream().into()
}

#[proc_macro_derive(OperonService, attributes(operon))]
pub fn derive_operon_service(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let attrs = extract_attr(&input.attrs, get_operon_attrs).unwrap_or_abort();

    let service = input.ident;
    let operon = attrs.crate_path.unwrap_or_else(|| operon_ident().into());
    let definition: syn::Path = match attrs.definition_path {
        Some(path) => parse_quote! { #path::schema },
        None => parse_quote! { schema },
    };

    quote! {
        impl #operon::service::OperonService for #service {
            type JobEnum = #definition::JobEnum;
            type ResolutionEnum = #definition::ResolutionEnum;
        }
    }
    .into()
}
