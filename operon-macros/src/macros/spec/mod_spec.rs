use indexmap::IndexSet;
use quote::quote;
use syn::parse_quote;

use crate::configs::AllConfig;
use crate::dependency_analysis::{
    get_direct_downstream_tasks, get_direct_upstream_tasks, get_tasks_repeating_on,
    get_upstream_tasks,
};
use crate::macros::spec::peer_txs::{impl_peer_txs, peer_txs_definition};
use crate::macros::spec::rebuilder::{impl_task_rebuilder, task_rebuilder_definition};
use crate::macros::spec::spec::{impl_spec_utils, impl_task_spec, task_spec_definition};
use crate::utils::operon_ident;

/// Generates the `mod spec` module containing the primary spec and task specs.
pub fn mod_spec(all_configs: &AllConfig) -> syn::ItemMod {
    let task_specs = all_configs.tasks.values().map(|task| {
        let upstream_tasks = get_direct_upstream_tasks(task, &all_configs.tasks);
        let downstream_tasks = get_direct_downstream_tasks(task, &all_configs.tasks);
        let all_upstream_tasks = get_upstream_tasks(task, &all_configs.tasks);
        let spawn_dim_repeating_tasks = task
            .spawn_dim
            .as_ref()
            .map(|spawn_dim| get_tasks_repeating_on(spawn_dim, &all_configs.tasks))
            .unwrap_or_default();
        let event_receiving_task_ids = downstream_tasks
            .iter()
            .chain(spawn_dim_repeating_tasks.iter())
            .map(|task| &task.id)
            .collect::<IndexSet<_>>();

        let task_spec_def = task_spec_definition(&task.id);
        let impl_task_spec = impl_task_spec(
            &all_configs.service_id,
            task,
            &all_upstream_tasks,
            &spawn_dim_repeating_tasks,
            &upstream_tasks,
            &downstream_tasks,
            &all_configs.entities,
            &all_configs.dimensions,
        );
        let impl_spec_utils = impl_spec_utils(&all_configs.service_id, task);

        let task_rebuilder_def = task_rebuilder_definition(task);
        let impl_task_rebuilder = impl_task_rebuilder(
            task,
            &spawn_dim_repeating_tasks,
            &downstream_tasks,
            &all_configs.tasks,
        );

        let peer_txs_def = peer_txs_definition(&task.id, &event_receiving_task_ids);
        let impl_peer_txs = impl_peer_txs(&task.id, &event_receiving_task_ids);

        quote! {
            #task_spec_def
            #impl_task_spec
            #impl_spec_utils

            #task_rebuilder_def
            #impl_task_rebuilder

            #peer_txs_def
            #impl_peer_txs
        }
    });

    let operon = operon_ident();

    parse_quote! {
        mod spec {
            use super::*;

            // Bring the metadata backend API traits into scope for the generated method calls.
            #[allow(unused_imports)]
            use #operon::__private::{
                MetaBackend, MetaClientApi, MetaConnApi, MetaResolutionApi, MetaTicketApi, MetaTxApi,
            };

            #(#task_specs)*
        }
    }
}
