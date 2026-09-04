use indexmap::IndexSet;
use syn::parse_quote;

use crate::utils::{
    job_enum_ident, operon_ident, peer_txs_ident, resolution_enum_ident, sender_ident,
    ticket_enum_ident,
};

/// Generates the peer event senders struct for a task.
///
/// # Example
/// ```rust,ignore
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/spec/peer_txs/peer_txs_definition.rs"))]
/// ```
pub fn peer_txs_definition(
    task_id: &syn::Ident,
    event_receiving_task_ids: &IndexSet<&syn::Ident>,
) -> syn::ItemStruct {
    let operon = operon_ident();
    let peer_txs_ident = peer_txs_ident(task_id);
    let job_enum_ident = job_enum_ident();
    let res_enum_ident = resolution_enum_ident();
    let ticket_enum_ident = ticket_enum_ident();

    let senders = event_receiving_task_ids
        .iter()
        .map(|downstream_task_id| sender_ident(downstream_task_id));

    parse_quote! {
        #[derive(Debug)]
        pub struct #peer_txs_ident {
            #(pub #senders: #operon::__private::PeerEventSender<schema::#job_enum_ident, schema::#res_enum_ident, schema::#ticket_enum_ident>,)*
        }
    }
}

#[cfg(test)]
mod tests {
    use quote::format_ident;
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;

    #[rstest]
    #[case::simple(
        format_ident!("beta"),
        vec![format_ident!("delta"), format_ident!("epsilon")],
        "spec/peer_txs/peer_txs_definition.rs"
    )]
    fn test_peer_txs_definition(
        #[case] task_id: syn::Ident,
        #[case] event_receiving_task_ids: Vec<syn::Ident>,
        #[case] fixture_path: &str,
    ) {
        let event_receiving_task_ids = event_receiving_task_ids.iter().collect::<IndexSet<_>>();

        let item = peer_txs_definition(&task_id, &event_receiving_task_ids);
        assert_item_eq(&item, fixture_path);
    }
}
