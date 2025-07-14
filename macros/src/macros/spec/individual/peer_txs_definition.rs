use indexmap::IndexSet;
use syn::parse_quote;

use crate::{
    configs::JobId,
    utils::{job_enum_ident, operon_ident, peer_txs_ident, resolution_enum_ident, sender_ident},
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
    use super::*;

    #[test]
    fn test_peer_txs_definition() {
        let beta = JobId::from("beta");
        let delta = JobId::from("delta");
        let epsilon = JobId::from("epsilon");
        let downstream_job_ids = IndexSet::from_iter([&delta, &epsilon]);

        let result = peer_txs_definition(&beta, &downstream_job_ids);
        let expected: syn::ItemStruct = parse_quote! {
            #[derive(Debug)]
            pub struct BetaPeerTxs {
                pub to_delta: operon::scheduler::PeerEventSender<schema::JobEnum, schema::ResolutionEnum>,
                pub to_epsilon: operon::scheduler::PeerEventSender<schema::JobEnum, schema::ResolutionEnum>,
            }
        };

        assert_eq!(result, expected);
    }
}
