use indexmap::IndexSet;
use syn::parse_quote;

use crate::configs::JobId;
use crate::utils::{
    job_enum_ident, operon_ident, peer_txs_ident, resolution_enum_ident, sender_ident,
};

/// Generates a struct definition for peer event senders.
///
/// Example:
/// ```rust,ignore
/// #[derive(Debug)]
/// pub struct BetaPeerEventSenders {
///     pub to_delta: operon::scheduler::PeerEventSender<schema::JobEnum, schema::ResolutionEnum>,
///     pub to_epsilon: operon::scheduler::PeerEventSender<schema::JobEnum, schema::ResolutionEnum>,
/// }
/// ```
pub fn peer_txs_definition(
    job_id: &JobId,
    event_receiving_job_ids: &IndexSet<&JobId>,
) -> syn::ItemStruct {
    let operon = operon_ident();
    let peer_txs_ident = peer_txs_ident(job_id);
    let job_enum_ident = job_enum_ident();
    let res_enum_ident = resolution_enum_ident();

    let senders = event_receiving_job_ids
        .iter()
        .map(|downstream_job_id| sender_ident(downstream_job_id));

    parse_quote! {
        #[derive(Debug)]
        pub struct #peer_txs_ident {
            #(pub #senders: #operon::scheduler::PeerEventSender<schema::#job_enum_ident, schema::#res_enum_ident>,)*
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;

    #[rstest]
    #[case::simple("beta", vec!["delta", "epsilon"], "spec/peer_txs_definition.rs")]
    fn test_peer_txs_definition(
        #[case] job_id: &str,
        #[case] event_receiving_job_ids: Vec<&str>,
        #[case] fixture_path: &str,
    ) {
        let job_id = JobId::from(job_id);
        let event_receiving_job_ids = event_receiving_job_ids
            .into_iter()
            .map(JobId::from)
            .collect::<Vec<_>>();
        let event_receiving_job_ids = event_receiving_job_ids.iter().collect::<IndexSet<_>>();

        let item = peer_txs_definition(&job_id, &event_receiving_job_ids);
        assert_item_eq(&item, fixture_path);
    }
}
