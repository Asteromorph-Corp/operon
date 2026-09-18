use indexmap::IndexSet;
use syn::parse_quote;

use crate::configs::TaskConfig;
use crate::utils::{
    dimension_metadata_ident, operon_ident, resolution_enum_ident, sender_ident, ticket_enum_ident,
    to_lit_str, to_pascal_case,
};

/// Generates the `on_receive_resolution` function for a task.
///
/// # Example
/// ```rust,ignore
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/spec/spec/fn_on_receive_resolution.rs"))]
/// ```
pub(super) fn fn_on_receive_resolution(
    task: &TaskConfig,
    downstream_tasks: &IndexSet<&TaskConfig>,
) -> syn::ImplItemFn {
    let operon = operon_ident();
    let res_enum_ident = resolution_enum_ident();
    let ticket_enum_ident = ticket_enum_ident();
    let task_id = to_lit_str(&task.id);

    let explode_arms = task.dims.iter().enumerate().map(|(idx, dim)| -> syn::Arm {
        let res_variant_ident = to_pascal_case(dim);
        let ticket_variant_ident = to_pascal_case(&task.id);
        let dim_meta = dimension_metadata_ident(dim);
        let send_explosions = downstream_tasks.iter().flat_map(|downstream_task| {
            let cnt = downstream_task.from.iter().filter(|arg| arg.id == task.to && arg.over.contains(dim)).count();
            let sender_ident = sender_ident(&downstream_task.id);
            let ok_msg = format!(
                "`{}` sent peer event to `{}`: {{resolution:?}}",
                task.id, downstream_task.id
            );
            let err_msg = format!(
                "`{}`'s peer channel closed before handling `{}`'s {{resolution:?}}",
                downstream_task.id, task.id
            );
            let dim_str = to_lit_str(dim);
            let stmt: syn::Stmt = parse_quote! {
                match peer_txs
                    .#sender_ident
                    .send(#operon::__private::PeerEvent::Explosion(#operon::__private::TicketExplosion {
                        ticket: schema::#ticket_enum_ident::#ticket_variant_ident(ticket),
                        dim: #dim_str,
                        ub: res.ub,
                    }))
                    .await
                {
                    Ok(_) => #operon::__private::tracing::trace!(#ok_msg),
                    Err(_) => #operon::__private::tracing::trace!(#err_msg),
                }
            };
            std::iter::repeat_n(stmt, cnt)
        });

        parse_quote! {
            schema::#res_enum_ident::#res_variant_ident(res) => {
                let affected = client.ticket(self.task_meta()).explode::<_, #idx>(metadata::#dim_meta(), res).await?;
                for ticket in affected {
                    #(#send_explosions)*
                }
            },
        }
    });

    parse_quote! {
        #[allow(unused_variables, unreachable_code, clippy::match_single_binding)]
        async fn on_receive_resolution(
            &self,
            client: MSto::Client<'_>,
            peer_txs: &Self::PeerEventSenders,
            resolution: schema::#res_enum_ident,
        ) -> Result<Vec<Self::Ticket>, #operon::error::SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
            match resolution {
                #(#explode_arms)*
                _ => return Err(#operon::error::SchedulerError::InvalidPeerEventReceived("resolution", #task_id)),
            }
            Ok(vec![])
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::configs::TaskConfigMap;
    use crate::dependency_analysis::get_direct_downstream_tasks;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::{all_tasks, task_delta};

    #[rstest]
    #[case::simple(task_delta(), "spec/spec/fn_on_receive_resolution.rs")]
    fn test_fn_on_receive_resolution(
        all_tasks: TaskConfigMap,
        #[case] task: TaskConfig,
        #[case] fixture_path: &str,
    ) {
        let downstream_tasks = get_direct_downstream_tasks(&task, &all_tasks);
        let item = fn_on_receive_resolution(&task, &downstream_tasks);
        assert_item_eq(&item, fixture_path);
    }
}
