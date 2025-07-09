use indexmap::IndexSet;
use syn::parse_quote;

use crate::{
    JobConfig,
    utils::{operon_ident, sender_ident},
};

pub(super) fn fn_send_on_finish(
    job: &JobConfig,
    spawn_dim_repeating_jobs: &IndexSet<&JobConfig>,
    downstream_jobs: &IndexSet<&JobConfig>,
) -> syn::ImplItemFn {
    let operon = operon_ident();

    let send_resolutions = spawn_dim_repeating_jobs
        .iter()
        .map(|repeating_job| -> syn::Expr {
            let sender_ident = sender_ident(&repeating_job.id);
            let ok_msg = format!(
                "`{}` sent peer event to `{}`: {{resolution:?}}",
                job.id, repeating_job.id
            );
            let err_msg = format!(
                "`{}`'s peer channel closed before handling `{}`'s {{resolution:?}}",
                repeating_job.id, job.id
            );

            parse_quote! {
                match peer_txs
                    .#sender_ident
                    .send(#operon::scheduler::PeerEvent::Resolution(resolution.into()))
                    .await
                {
                    Ok(_) => #operon::log::trace!(#ok_msg),
                    Err(_) => #operon::log::trace!(#err_msg),
                }
            }
        });

    let send_jobs = downstream_jobs.iter().map(|downstream_job| -> syn::Expr {
        let sender_ident = sender_ident(&downstream_job.id);
        let ok_msg = format!(
            "`{}` sent peer event to `{}`: {{job:?}}",
            job.id, downstream_job.id
        );
        let err_msg = format!(
            "`{}`'s peer channel closed before handling `{}`'s {{job:?}}",
            downstream_job.id, job.id
        );

        parse_quote! {
            match peer_txs
                .#sender_ident
                .send(#operon::scheduler::PeerEvent::Job(job.into()))
                .await
            {
                Ok(_) => #operon::log::trace!(#ok_msg),
                Err(_) => #operon::log::trace!(#err_msg),
            }
        }
    });

    parse_quote! {
        async fn send_on_finish(
            &self,
            peer_txs: &Self::PeerEventSenders,
            job: Self::Job,
            resolution: Self::Resolution,
        ) -> Result<(), #operon::scheduler::SchedulerError> {
            #(#send_resolutions)*
            #(#send_jobs)*
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;

    use crate::JobArg;

    use super::*;

    #[test]
    fn test_fn_send_on_finish() {
        let job = JobConfig {
            id: "beta".to_string(),
            from: vec![JobArg {
                id: "a".to_string(),
                over: vec![],
            }],
            to: "b".to_string(),
            dims: vec!["i".to_string()],
            spawn_dim: Some("j".to_string()),
        };
        let delta = JobConfig {
            id: "delta".to_string(),
            from: vec![
                JobArg {
                    id: "a".to_string(),
                    over: vec![],
                },
                JobArg {
                    id: "b".to_string(),
                    over: vec![],
                },
                JobArg {
                    id: "c".to_string(),
                    over: vec![],
                },
            ],
            to: "d".to_string(),
            dims: vec!["i".to_string(), "j".to_string(), "k".to_string()],
            spawn_dim: None,
        };
        let epsilon = JobConfig {
            id: "epsilon".to_string(),
            from: vec![
                JobArg {
                    id: "b".to_string(),
                    over: vec!["j".to_string()],
                },
                JobArg {
                    id: "d".to_string(),
                    over: vec!["j".to_string()],
                },
            ],
            to: "e".to_string(),
            dims: vec!["i".to_string(), "k".to_string()],
            spawn_dim: None,
        };
        let spawn_dim_repeating_jobs = IndexSet::from_iter([&delta]);
        let downstream_jobs = IndexSet::from_iter([&delta, &epsilon]);

        let item = fn_send_on_finish(&job, &spawn_dim_repeating_jobs, &downstream_jobs);
        let expected: syn::ImplItemFn = parse_quote! {
            async fn send_on_finish(
                &self,
                peer_txs: &Self::PeerEventSenders,
                job: Self::Job,
                resolution: Self::Resolution,
            ) -> Result<(), operon::scheduler::SchedulerError> {
                match peer_txs
                    .to_delta
                    .send(operon::scheduler::PeerEvent::Resolution(resolution.into()))
                    .await
                {
                    Ok(_) => operon::log::trace!("`beta` sent peer event to `delta`: {resolution:?}"),
                    // Verbosity should be low here, since this can happen
                    // an arbitrary number of times
                    // if a descendant scheduler errored out.
                    Err(_) => operon::log::trace!(
                        "`delta`'s peer channel closed before handling `beta`'s {resolution:?}"
                    ),
                }
                // out-dependencies (delta, epsilon)
                match peer_txs
                    .to_delta
                    .send(operon::scheduler::PeerEvent::Job(job.into()))
                    .await
                {
                    Ok(_) => operon::log::trace!("`beta` sent peer event to `delta`: {job:?}"),
                    Err(_) => operon::log::trace!(
                        "`delta`'s peer channel closed before handling `beta`'s {job:?}"
                    ),
                }
                match peer_txs
                    .to_epsilon
                    .send(operon::scheduler::PeerEvent::Job(job.into()))
                    .await
                {
                    Ok(_) => operon::log::trace!("`beta` sent peer event to `epsilon`: {job:?}"),
                    Err(_) => operon::log::trace!(
                        "`epsilon`'s peer channel closed before handling `beta`'s {job:?}"
                    ),
                }
                Ok(())
            }
        };

        assert_eq!(
            item.to_token_stream().to_string(),
            expected.to_token_stream().to_string()
        );
    }
}
