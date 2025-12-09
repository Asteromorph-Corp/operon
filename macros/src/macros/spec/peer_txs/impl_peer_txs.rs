use indexmap::IndexSet;
use syn::parse_quote;

use crate::utils::{
    job_enum_ident, job_id_ident, operon_ident, peer_txs_ident, resolution_enum_ident, sender_ident,
};

/// Generates an implementation of `PeerEventSenders` for a job's peer event senders.
///
/// # Example
/// ```rust,ignore
/// #[operon::async_trait::async_trait]
/// #[automatically_derived]
/// impl operon::scheduler::PeerEventSenders<schema::JobEnum, schema::ResolutionEnum> for BetaPeerTxs {
///     fn gather_from(
///         mut senders: operon::scheduler::PeerEventSenderMap<schema::JobEnum, schema::ResolutionEnum>,
///     ) -> Self {
///         BetaPeerTxs {
///             to_delta: senders
///                 .remove(metadata::DELTA_ID)
///                 .unwrap_or_else(|| panic!("No sender for job `{}` found", metadata::DELTA_ID)),
///             to_epsilon: senders
///                 .remove(metadata::EPSILON_ID)
///                 .unwrap_or_else(|| panic!("No sender for job `{}` found", metadata::EPSILON_ID)),
///         }
///     }
///
///     fn downgrade_all(&mut self) {
///         self.to_delta.downgrade();
///         self.to_epsilon.downgrade();
///     }
/// }
/// ```
pub fn impl_peer_txs(
    job_id: &syn::Ident,
    event_receiving_job_ids: &IndexSet<&syn::Ident>,
) -> syn::ItemImpl {
    let operon = operon_ident();
    let peer_txs_ident = peer_txs_ident(job_id);
    let job_enum_ident = job_enum_ident();
    let res_enum_ident = resolution_enum_ident();

    let sender_value = event_receiving_job_ids
        .iter()
        .map(|downstream_job_id| -> syn::FieldValue {
            let sender_ident = sender_ident(downstream_job_id);
            let downstream_job_id_ident = job_id_ident(downstream_job_id);

            parse_quote! {
                #sender_ident: senders
                    .remove(metadata::#downstream_job_id_ident)
                    .unwrap_or_else(|| {
                        panic!("No sender for job `{}` found", metadata::#downstream_job_id_ident)
                    })
            }
        });

    let senders = event_receiving_job_ids
        .iter()
        .map(|downstream_job_id| sender_ident(downstream_job_id));

    parse_quote! {
        #[#operon::async_trait::async_trait]
        #[automatically_derived]
        impl #operon::scheduler::PeerEventSenders<schema::#job_enum_ident, schema::#res_enum_ident> for #peer_txs_ident {
            fn gather_from(
                mut senders: #operon::scheduler::PeerEventSenderMap<
                    schema::#job_enum_ident,
                    schema::#res_enum_ident
                >,
            ) -> Self {
                #peer_txs_ident {
                    #(#sender_value,)*
                }
            }

            fn downgrade_all(&mut self) {
                #(self.#senders.downgrade();)*
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
        #[case] job_id: syn::Ident,
        #[case] event_receiving_job_ids: Vec<syn::Ident>,
        #[case] fixture_path: &str,
    ) {
        let event_receiving_job_ids = event_receiving_job_ids.iter().collect::<IndexSet<_>>();
        let item = impl_peer_txs(&job_id, &event_receiving_job_ids);
        assert_item_eq(&item, fixture_path);
    }
}
