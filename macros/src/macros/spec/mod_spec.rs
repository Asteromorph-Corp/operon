use indexmap::IndexSet;
use quote::quote;
use syn::parse_quote;

use crate::AllConfig;
use crate::macros::spec::individual::{
    impl_job_rebuilder, impl_job_spec, impl_peer_txs, job_rebuilder_definition,
    job_spec_definition, peer_txs_definition,
};
use crate::macros::spec::primary::{impl_primary_spec, primary_spec_definition};
use crate::utils::{get_direct_downstream_jobs, get_direct_upstream_jobs, get_jobs_repeating_on};

/// Generates the `mod spec` module containing the primary spec and job specs.
pub fn mod_spec(all_configs: &AllConfig) -> syn::ItemMod {
    let primary_spec = primary_spec_definition();
    let impl_primary_spec = impl_primary_spec(
        &all_configs.service_id,
        &all_configs.primary_entity,
        &all_configs.primary_dimension,
    );

    let job_specs = all_configs.jobs.values().map(|job| {
        let upstream_jobs = get_direct_upstream_jobs(job, &all_configs.jobs);
        let downstream_jobs = get_direct_downstream_jobs(job, &all_configs.jobs);
        let spawn_dim_repeating_jobs = job
            .spawn_dim
            .as_ref()
            .map(|spawn_dim| get_jobs_repeating_on(spawn_dim, &all_configs.jobs))
            .unwrap_or_default();
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

        let job_rebuilder_def = job_rebuilder_definition(job);
        let impl_job_rebuilder = impl_job_rebuilder(
            job,
            &all_configs.primary_dimension,
            &spawn_dim_repeating_jobs,
            &downstream_jobs,
        );

        let peer_txs_def = peer_txs_definition(&job.id, &event_receiving_job_ids);
        let impl_peer_txs = impl_peer_txs(&job.id, &event_receiving_job_ids);

        quote! {
            #job_spec_def
            #impl_job_spec

            #job_rebuilder_def
            #impl_job_rebuilder

            #peer_txs_def
            #impl_peer_txs
        }
    });

    parse_quote! {
        mod spec {
            use super::*;

            #primary_spec
            #impl_primary_spec

            #(#job_specs)*
            // Add your spec-related items here
        }
    }
}
