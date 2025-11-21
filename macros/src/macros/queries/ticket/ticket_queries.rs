use indexmap::IndexSet;
use quote::quote;

use crate::configs::{DimensionConfig, JobConfig};
use crate::macros::queries::ticket::fn_explode::fn_explode;
use crate::macros::queries::ticket::fn_raise_quota::fn_raise_quota;

/// Generates all resolution-related queries for a given dimension.
pub fn ticket_queries(
    job: &JobConfig,
    job_dims: &[&DimensionConfig],
    quota_required_dims: &IndexSet<&DimensionConfig>,
) -> proc_macro2::TokenStream {
    let raise_quota_fns = quota_required_dims
        .iter()
        .map(|dim| fn_raise_quota(job, dim));
    let explode_fns = job_dims
        .iter()
        .enumerate()
        .map(|(idx, dim)| fn_explode(job, dim, idx));

    quote! {
        #(#explode_fns)*
        #(#raise_quota_fns)*
    }
}
