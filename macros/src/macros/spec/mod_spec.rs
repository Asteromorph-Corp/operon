use quote::quote;
use syn::parse_quote;

use crate::{
    AllConfig,
    macros::spec::{
        individual::{
            impl_job_rebuilder, impl_job_spec, impl_peer_txs, job_rebuilder_definition,
            job_spec_definition, peer_txs_definition,
        },
        primary::{impl_primary_spec, primary_spec_definition},
    },
    utils::get_downstream_jobs,
};

pub fn mod_spec(all_configs: &AllConfig) -> syn::ItemMod {
    let primary_spec = primary_spec_definition();
    let impl_primary_spec = impl_primary_spec(
        &all_configs.service_id,
        &all_configs.primary_entity,
        &all_configs.primary_dimension,
    );

    let job_specs = all_configs.jobs.values().map(|job| {
        let downstream_jobs = get_downstream_jobs(&job.id, &all_configs.jobs);

        let job_spec_def = job_spec_definition(&job.id);
        let impl_job_spec = impl_job_spec(&all_configs.service_id, job, &all_configs.jobs);

        let job_rebuilder_def = job_rebuilder_definition(job);
        let impl_job_rebuilder = impl_job_rebuilder(job);

        let peer_txs_def = peer_txs_definition(&job.id, &downstream_jobs);
        let impl_peer_txs = impl_peer_txs(&job.id, &downstream_jobs);

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
