use indexmap::IndexSet;
use quote::quote;
use syn::parse_quote;

use crate::configs::AllConfig;
use crate::dependency_analysis::{
    get_direct_downstream_jobs, get_direct_upstream_jobs, get_jobs_repeating_on, get_upstream_jobs,
};
use crate::macros::spec::peer_txs::{impl_peer_txs, peer_txs_definition};
use crate::macros::spec::rebuilder::{impl_job_rebuilder, job_rebuilder_definition};
use crate::macros::spec::spec::{impl_job_spec, impl_spec_utils, job_spec_definition};

/// Generates the `mod spec` module containing the primary spec and job specs.
pub fn mod_spec(all_configs: &AllConfig) -> syn::ItemMod {
    let job_specs = all_configs.jobs.values().map(|job| {
        let upstream_jobs = get_direct_upstream_jobs(job, &all_configs.jobs);
        let downstream_jobs = get_direct_downstream_jobs(job, &all_configs.jobs);
        let all_upstream_jobs = get_upstream_jobs(job, &all_configs.jobs);
        let spawn_dim_repeating_jobs = job
            .spawn_dim
            .as_ref()
            .map(|spawn_dim| get_jobs_repeating_on(spawn_dim, &all_configs.jobs))
            .unwrap_or_default();
        let spawn_dim_repeating_job_downstream_jobs = spawn_dim_repeating_jobs
            .iter()
            .flat_map(|job| get_direct_downstream_jobs(job, &all_configs.jobs))
            .collect::<Vec<_>>();
        let event_receiving_job_ids = downstream_jobs
            .iter()
            .chain(spawn_dim_repeating_jobs.iter())
            .map(|job| &job.id)
            .collect::<IndexSet<_>>();

        let job_spec_def = job_spec_definition(&job.id);
        let impl_job_spec = impl_job_spec(
            &all_configs.service_id,
            job,
            &spawn_dim_repeating_jobs,
            &upstream_jobs,
            &downstream_jobs,
            &all_configs.entities,
            &all_configs.dimensions,
        );
        let impl_spec_utils = impl_spec_utils(&all_configs.service_id, job, &all_upstream_jobs);

        let job_rebuilder_def = job_rebuilder_definition(job);
        let impl_job_rebuilder = impl_job_rebuilder(
            job,
            &spawn_dim_repeating_jobs,
            &downstream_jobs,
            &spawn_dim_repeating_job_downstream_jobs,
        );

        let peer_txs_def = peer_txs_definition(&job.id, &event_receiving_job_ids);
        let impl_peer_txs = impl_peer_txs(&job.id, &event_receiving_job_ids);

        quote! {
            #job_spec_def
            #impl_job_spec
            #impl_spec_utils

            #job_rebuilder_def
            #impl_job_rebuilder

            #peer_txs_def
            #impl_peer_txs
        }
    });

    parse_quote! {
        mod spec {
            use super::*;

            #(#job_specs)*
        }
    }
}
