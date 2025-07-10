use indexmap::IndexSet;
use syn::parse_quote;

use crate::{
    configs::JobId,
    utils::{
        job_enum_ident, job_ident, operon_ident, peer_txs_ident, resolution_enum_ident,
        sender_ident,
    },
};

/// Generates an implementation of `PeerEventSenders` for a job's peer event senders.
///
/// Example:
/// ```rust,ignore
/// #[operon::async_trait::async_trait]
/// #[automatically_derived]
/// impl operon::scheduler::PeerEventSenders<schema::JobEnum, schema::ResolutionEnum> for BetaPeerTxs {
///     fn gather_from(
///         mut senders: operon::scheduler::PeerEventSenderMap<schema::JobEnum, schema::ResolutionEnum>,
///     ) -> Self {
///         BetaPeerTxs {
///             to_delta: senders.remove(<schema::DeltaJob as operon::scheduler::Job>::id()).unwrap_or_else(|| {
///                 panic!("No sender for job `{}` found", <schema::DeltaJob as operon::scheduler::Job>::id())
///             }),
///             to_epsilon: senders.remove(<schema::EpsilonJob as operon::scheduler::Job>::id()).unwrap_or_else(|| {
///                 panic!("No sender for job `{}` found", <schema::EpsilonJob as operon::scheduler::Job>::id())
///             }),
///         }
///     }
///
///     fn downgrade_all(&mut self) {
///         self.to_delta.downgrade();
///         self.to_epsilon.downgrade();
///     }
/// }
/// ```
pub fn impl_peer_txs(job_id: &JobId, event_receiving_job_ids: &IndexSet<&JobId>) -> syn::ItemImpl {
    let operon = operon_ident();
    let peer_txs_ident = peer_txs_ident(job_id);
    let job_enum_ident = job_enum_ident();
    let res_enum_ident = resolution_enum_ident();

    let sender_value = event_receiving_job_ids.iter().map(|downstream_job_id| -> syn::FieldValue {
        let sender_ident = sender_ident(downstream_job_id);
        let job_ident = job_ident(downstream_job_id);

        parse_quote! {
            #sender_ident: senders
                .remove(<schema::#job_ident as #operon::schema_base::Job>::id())
                .unwrap_or_else(|| {
                    panic!("No sender for job `{}` found", <schema::#job_ident as #operon::schema_base::Job>::id())
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
    use super::*;

    #[test]
    fn test_impl_peer_event_senders() {
        let beta = JobId::from("beta");
        let delta = JobId::from("delta");
        let epsilon = JobId::from("epsilon");
        let downstream_jobs = IndexSet::from_iter([&delta, &epsilon]);

        let result = impl_peer_txs(&beta, &downstream_jobs);
        let expected: syn::ItemImpl = parse_quote! {
            #[operon::async_trait::async_trait]
            #[automatically_derived]
            impl operon::scheduler::PeerEventSenders<schema::JobEnum, schema::ResolutionEnum> for BetaPeerTxs {
                fn gather_from(
                    mut senders: operon::scheduler::PeerEventSenderMap<schema::JobEnum, schema::ResolutionEnum>,
                ) -> Self {
                    BetaPeerTxs {
                        to_delta: senders.remove(<schema::DeltaJob as operon::schema_base::Job>::id()).unwrap_or_else(|| {
                            panic!("No sender for job `{}` found", <schema::DeltaJob as operon::schema_base::Job>::id())
                        }),
                        to_epsilon: senders.remove(<schema::EpsilonJob as operon::schema_base::Job>::id()).unwrap_or_else(|| {
                            panic!("No sender for job `{}` found", <schema::EpsilonJob as operon::schema_base::Job>::id())
                        }),
                    }
                }

                fn downgrade_all(&mut self) {
                    self.to_delta.downgrade();
                    self.to_epsilon.downgrade();
                }
            }
        };

        assert_eq!(result, expected);
    }
}
