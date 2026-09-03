use indexmap::IndexSet;
use syn::parse_quote;

use crate::utils::{
    job_enum_ident, operon_ident, peer_txs_ident, resolution_enum_ident, sender_ident,
    task_id_ident, ticket_enum_ident,
};

/// Generates an implementation of `PeerEventSenders` trait for a task's peer event senders.
///
/// # Example
/// ```rust,ignore
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/spec/peer_txs/impl_peer_txs.rs") )]
/// ```
pub fn impl_peer_txs(
    task_id: &syn::Ident,
    event_receiving_task_ids: &IndexSet<&syn::Ident>,
) -> syn::ItemImpl {
    let operon = operon_ident();
    let peer_txs_ident = peer_txs_ident(task_id);
    let job_enum_ident = job_enum_ident();
    let res_enum_ident = resolution_enum_ident();
    let ticket_enum_ident = ticket_enum_ident();

    let sender_value = event_receiving_task_ids.iter().map(
        |downstream_task_id| -> syn::FieldValue {
            let sender_ident = sender_ident(downstream_task_id);
            let downstream_task_id_ident = task_id_ident(downstream_task_id);

            parse_quote! {
                #sender_ident: senders
                    .remove(metadata::#downstream_task_id_ident)
                    .unwrap_or_else(|| {
                        panic!("No sender for task `{}` found", metadata::#downstream_task_id_ident)
                    })
            }
        },
    );

    parse_quote! {
        #[#operon::__private::async_trait::async_trait]
        #[automatically_derived]
        impl #operon::__private::PeerEventSenders<schema::#job_enum_ident, schema::#res_enum_ident, schema::#ticket_enum_ident> for #peer_txs_ident {
            fn gather_from(
                mut senders: #operon::__private::PeerEventSenderMap<
                    schema::#job_enum_ident,
                    schema::#res_enum_ident,
                    schema::#ticket_enum_ident,
                >,
            ) -> Self {
                #peer_txs_ident {
                    #(#sender_value,)*
                }
            }
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
        "spec/peer_txs/impl_peer_txs.rs"
    )]
    fn test_impl_peer_event_senders(
        #[case] task_id: syn::Ident,
        #[case] event_receiving_task_ids: Vec<syn::Ident>,
        #[case] fixture_path: &str,
    ) {
        let event_receiving_task_ids = event_receiving_task_ids.iter().collect::<IndexSet<_>>();
        let item = impl_peer_txs(&task_id, &event_receiving_task_ids);
        assert_item_eq(&item, fixture_path);
    }
}
