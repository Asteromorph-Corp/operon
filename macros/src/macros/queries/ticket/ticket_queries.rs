use quote::quote;

use crate::DimensionConfig;
use crate::configs::{DimensionConfigMap, JobConfig};
use crate::dependency_analysis::get_quota_required_dims;
use crate::macros::queries::ticket::fn_clear_ticket::fn_clear_ticket;
use crate::macros::queries::ticket::fn_explode::fn_explode;
use crate::macros::queries::ticket::fn_get_all::fn_get_all;
use crate::macros::queries::ticket::fn_init_ticket::fn_init_ticket;
use crate::macros::queries::ticket::fn_mark_done::fn_mark_done;
use crate::macros::queries::ticket::fn_put_ticket::fn_put_ticket;
use crate::macros::queries::ticket::fn_raise_dep::fn_raise_dep;
use crate::macros::queries::ticket::fn_resolve_dep::fn_resolve_dep;

/// Generates all resolution-related queries for a given dimension.
pub fn ticket_queries(
    job: &JobConfig,
    job_dims: &[&DimensionConfig],
    all_dims: &DimensionConfigMap,
) -> proc_macro2::TokenStream {
    let init_fn = fn_init_ticket(job);
    let clear_fn = fn_clear_ticket(job);
    let put_fn = fn_put_ticket(job);
    let get_all_fn = fn_get_all(job);
    let mark_done_fn = fn_mark_done(job);
    let raise_dep_fn = fn_raise_dep(job);

    let resolve_dep_fns = get_quota_required_dims(job, all_dims)
        .into_iter()
        .map(|dim| fn_resolve_dep(job, dim));
    let explode_fns = job_dims.iter().map(|dim| fn_explode(job, dim));

    quote! {
        #init_fn
        #clear_fn
        #put_fn
        #get_all_fn
        #mark_done_fn
        #raise_dep_fn
        #(#resolve_dep_fns)*
        #(#explode_fns)*
    }
}
