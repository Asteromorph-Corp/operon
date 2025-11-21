use quote::quote;

use crate::configs::{DimensionConfig, JobConfig};
use crate::macros::queries::ticket::fn_explode::fn_explode;

/// Generates all resolution-related queries for a given dimension.
pub fn ticket_queries(job: &JobConfig, job_dims: &[&DimensionConfig]) -> proc_macro2::TokenStream {
    let explode_fns = job_dims
        .iter()
        .enumerate()
        .map(|(idx, dim)| fn_explode(job, dim, idx));

    quote! {
        #(#explode_fns)*
    }
}
