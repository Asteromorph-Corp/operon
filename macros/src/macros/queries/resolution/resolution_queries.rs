use quote::quote;

use crate::{
    configs::DimensionConfig,
    macros::queries::resolution::{
        fn_clear_resolution::fn_clear_resolution, fn_get_resolution::fn_get_resolution,
        fn_init_resolution::fn_init_resolution, fn_put_resolution::fn_put_resolution,
    },
};

/// Generates all resolution-related queries for a given dimension.
pub fn resolution_queries(dimension: &DimensionConfig) -> proc_macro2::TokenStream {
    let init_fn = fn_init_resolution(dimension);
    let clear_fn = fn_clear_resolution(dimension);
    let get_fn = fn_get_resolution(dimension);
    let put_fn = fn_put_resolution(dimension);

    quote! {
        #init_fn
        #clear_fn
        #get_fn
        #put_fn
    }
}
