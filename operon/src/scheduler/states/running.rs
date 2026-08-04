use std::collections::HashSet;

use async_trait::async_trait;
use tokio::task::JoinSet;
use uuid::Uuid;

use crate::meta_storage::{MetaBackend, MetaClientApi, MetaConnApi, MetaTxApi};
use crate::scheduler::SchedulerError;
use crate::scheduler::context::SchedulerContext;
use crate::scheduler::events::{ControlEvent, IndividualControlEvent};
use crate::scheduler::spec::ControlChannel;
use crate::scheduler::states::{NextState, SchedulerState};
use crate::schema::{RunFootprint, RunState};
use crate::service::OperonService;
use crate::storage::OperonStorage;

pub struct RunningState<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    ctx: SchedulerContext<Svc, Sto, MSto>,
    run_id: Uuid,
    execution_id: Uuid,
    handles: JoinSet<()>,
    ctrl_channels: Vec<ControlChannel>,
    force_exited: bool,
    exit_ui: bool,
    time_started: std::time::Instant,
}

impl<Svc, Sto, MSto> RunningState<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    pub fn new(
        ctx: SchedulerContext<Svc, Sto, MSto>,
        channel_size: usize,
        run_id: Uuid,
        execution_id: Uuid,
        clean: bool,
    ) -> Self {
        let (handles, ctrl_channels) = ctx.handler.prepare_channels(channel_size).run_schedulers(
            &ctx.service,
            &ctx.storage,
            &ctx.meta_storage,
            &ctx.progresses,
            clean,
        );

        Self {
            ctx,
            run_id,
            execution_id,
            handles,
            ctrl_channels,
            force_exited: false,
            exit_ui: false,
            time_started: std::time::Instant::now(),
        }
    }

    async fn handle_quit(&mut self, force: bool, exit_ui: bool) {
        self.force_exited = force;
        self.exit_ui = exit_ui;
        for channel in &self.ctrl_channels {
            let _ = channel
                .tx
                .send(IndividualControlEvent::Quit { force })
                .await;
        }
    }
}

#[async_trait]
impl<Svc, Sto, MSto> SchedulerState for RunningState<Svc, Sto, MSto>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    type Error = SchedulerError<Svc::Error, Sto::Error, MSto::Error>;

    /// The running state only progresses into the `Exit` state, when all individual schedulers ran
    /// to completion, when a stopping command was issued, or when a panic occurred in one of the
    /// individual schedulers.
    /// (Errors are non-fatal to the scheduler, as execution should continue for the non-erroring
    /// schedulers.)
    ///
    /// This method therefore waits for all handles and early-returns an error if any of the handles
    /// panicked.
    async fn wait_progress(&mut self) -> Result<(), Self::Error> {
        while let Some(joined) = self.handles.join_next().await {
            joined?;
        }
        Ok(())
    }

    async fn handle_progress(self: Box<Self>) -> Result<NextState<Self::Error>, Self::Error> {
        let snapshot = self.ctx.progresses.snapshot().await;

        let state = if snapshot.all_finished() {
            RunState::Completed
        } else if snapshot.all_stopped() && !self.force_exited {
            RunState::Stopped
        } else {
            RunState::Aborted
        };

        let footprint = RunFootprint::new(self.run_id, state);

        // Write the footprint to the data storage...
        self.ctx.storage.put_footprint(&footprint).await?;

        // ...and to the metadata storage.
        let mut conn = self.ctx.meta_storage.scheduler_conn().await?;
        let tx = conn.transaction().await?;
        tx.as_client().upsert_run(&footprint).await?;
        tx.as_client()
            .update_execution_on_finish(&footprint, self.execution_id)
            .await?;
        tx.commit().await?;

        tracing::info!("Run completed in {:?}.", self.time_started.elapsed());

        Ok(NextState::Exit {
            exit_ui: self.exit_ui,
        })
    }

    async fn handle_control_event(
        mut self: Box<Self>,
        evt: ControlEvent,
    ) -> Result<NextState<Self::Error>, Self::Error> {
        // TODO: make `RunningState` aware of individual scheduler states and decide whether to
        // propagate events.

        if let ControlEvent::Quit { force, no_exit } = evt {
            self.force_exited = force;
            self.exit_ui = !no_exit;
        };

        let snapshot = self.ctx.progresses.snapshot().await;

        match evt {
            ControlEvent::Run { .. } => tracing::warn!(
                "Already run. Use `exit` or `quit` to terminate the current session before starting a new run."
            ),
            ControlEvent::Check { .. } => {
                tracing::warn!("Cannot check after the run has already started.")
            }
            ControlEvent::Quit { force, .. } if snapshot.any_error() => {
                if !force {
                    tracing::warn!(
                        "Cannot stop due to previous errors. Defaulting to a force quit."
                    );
                }
                self.handle_quit(true, false).await;
            }
            ControlEvent::Quit { force, no_exit } => {
                if force {
                    tracing::warn!("Sent abort request, stopping immediately...");
                } else {
                    tracing::info!("Sent stop request, stopping gracefully...");
                }
                self.handle_quit(force, !no_exit).await;
            }
            ControlEvent::Pause { .. } if !snapshot.any_running() => {
                tracing::warn!(
                    "No running jobs to pause, did you mean to `exit` or `quit` instead?"
                )
            }
            ControlEvent::Pause { targets, cascade } => {
                let targets: HashSet<String> = HashSet::from_iter(targets);

                for channel in &self.ctrl_channels {
                    let is_target = targets.is_empty() || targets.contains(channel.job_id);
                    let is_downstream =
                        cascade && channel.upstream_jobs.iter().any(|id| targets.contains(*id));

                    if is_target || is_downstream {
                        let _ = channel.tx.send(IndividualControlEvent::Pause).await;
                    }
                }
            }
            ControlEvent::Resume { .. } if !snapshot.any_paused() => {
                tracing::warn!("No paused jobs to resume.")
            }
            ControlEvent::Resume { targets } => {
                let targets: HashSet<String> = HashSet::from_iter(targets);

                for channel in &self.ctrl_channels {
                    if targets.is_empty() || targets.contains(channel.job_id) {
                        let _ = channel.tx.send(IndividualControlEvent::Resume).await;
                    }
                }
            }
            ControlEvent::Exit => tracing::warn!(
                "Cannot exit while jobs are running or paused. \
                Use `quit` for a graceful stop, or `quit --force` to abort all jobs."
            ),
        }

        Ok(NextState::Next(self))
    }
}
