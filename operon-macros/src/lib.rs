//! # operon-macros
//!
//! Macros for the Operon crate.

#![warn(unreachable_pub)]
#![warn(unused_lifetimes)]
#![warn(unused_qualifications)]
#![warn(single_use_lifetimes)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]
#![warn(noop_method_call)]
#![warn(let_underscore_drop)]
#![warn(meta_variable_misuse)]
#![warn(unused_results)]

mod configs;
mod dependency_analysis;
mod macros;
mod utils;

#[cfg(test)]
mod test_utils;

use proc_macro::TokenStream;
use proc_macro_error::{ResultExt, proc_macro_error};
use quote::{ToTokens, quote};
use syn::{DeriveInput, parse_macro_input, parse_quote};

use crate::macros::operon;
use crate::utils::{extract_attr, get_operon_attrs, operon_ident};

// TODO: use text fixtures instead of constructing the configs in code.

/// Define the pipeline in the Operon DSL.
#[proc_macro]
#[proc_macro_error]
pub fn define_operon(input: TokenStream) -> TokenStream {
    let all_configs: configs::AllConfig = match syn::parse(input) {
        Ok(config) => config,
        Err(e) => return e.to_compile_error().into(),
    };
    let operon = operon(&all_configs);
    operon.into_token_stream().into()
}

/// Derives `OperonService` for a pipeline service type.
///
/// # Attributes
///
/// - `#[operon(error = MyError)]`: the service's error type `Self::Error`, written as a type
///   (default `::operon::error::UserError`)
/// - `#[operon(defined_at = "path")]`: path to where `define_operon!` was invoked (default `self`)
/// - `#[operon(crate = "path")]`: path to the `operon` crate (default `::operon`)
#[proc_macro_derive(OperonService, attributes(operon))]
#[proc_macro_error]
pub fn derive_operon_service(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let attrs = extract_attr(&input.attrs, get_operon_attrs).unwrap_or_abort();

    let service = input.ident;
    let operon = attrs.crate_path.unwrap_or_else(|| operon_ident().into());
    let definition = attrs.definition_path.unwrap_or(parse_quote!(self));
    let error_ty: syn::Type = attrs
        .error_type
        .unwrap_or_else(|| parse_quote!(#operon::error::UserError));

    quote! {
        #[automatically_derived]
        impl #operon::OperonService for #service {
            type Error = #error_ty;
            type JobEnum = #definition::schema::JobEnum;
            type ResolutionEnum = #definition::schema::ResolutionEnum;
            type TicketEnum = #definition::schema::TicketEnum;
        }

        #[automatically_derived]
        impl<Sto: #definition::__misc::StorageTrait> #operon::__private::ValidOperon<#service, Sto>
            for (#service, Sto)
        {
            fn scheduler_handler<MSto: #operon::__private::MetaBackend>()
            -> #operon::__private::SchedulerHandler<#service, Sto, MSto> {
                #definition::__misc::scheduler_handler::<#service, Sto, MSto>()
            }
        }
    }
    .into()
}
