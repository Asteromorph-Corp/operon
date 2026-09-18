use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;

use crate::MemMetaStorage;
use crate::meta_storage::{
    MemClient, MetaBackend, MetaClientApi, MetaResolutionApi, MetaTicketApi,
};
use crate::scheduler::events::{
    IndividualControlEventReceiver, ServicePeerEventReceiver, ServicePeerEventSenderMap,
};
use crate::scheduler::individual_scheduler::IndividualScheduler;
use crate::scheduler::spec::SpecWithMetadata;
use crate::scheduler::{SchedulerError, TaskRebuilder, TaskSpec};
use crate::schema::{CheckMode, Job, SharedProgress, TableShape, Ticket};
use crate::service::OperonService;
use crate::storage::OperonStorage;

/// Rebuilds a task's handler against the in-memory backend, for the scratch store a staged
/// rebuild replays into.
pub trait ToMemHandler<Svc, Sto>: Send + Sync + 'static
where
    Svc: OperonService,
    Sto: OperonStorage,
{
    /// Converts the task handler to an in-memory version.
    fn to_mem(&self) -> Box<dyn TaskHandler<Svc, Sto, MemMetaStorage>>;
}

#[async_trait]
/// The dyn-compatible face of one task's [`TaskSpec`], carrying the arity `N` as a type parameter
/// would not.
///
/// Implemented for every [`SpecWithMetadata`], so that the scheduler holds the tasks of a pipeline
/// in one collection despite their differing arities.
/// It is what the scheduler prepares a task's metadata and starts its individual scheduler through.
pub trait TaskHandler<Svc, Sto, MSto>: ToMemHandler<Svc, Sto> + Send + Sync + 'static
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
{
    /// The id of the task this handler runs.
    fn task_id(&self) -> &'static str;

    /// This task's id and the ids of every task it transitively depends on, in lexicographic
    /// order.
    fn all_upstream_tasks(&self) -> Vec<&'static str>;

    /// How many of this task's jobs may run at once.
    fn pool_size(&self) -> usize {
        1
    }

    /// Initialize the fact storage for the primary resolution.
    ///
    /// This function should be idempotent,
    /// i.e. calling it multiple times, or calling it on an already-initialized storage should do
    /// nothing.
    ///
    /// Returns the shape the resolution table was found under.
    async fn init_resolution(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<TableShape, SchedulerError<Svc::Error, Sto::Error, MSto::Error>>;

    /// Clear the primary resolution from the fact storage, assuming the table is already
    /// initialized.
    async fn clear_resolution(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<(), SchedulerError<Svc::Error, Sto::Error, MSto::Error>>;

    /// Initialize the ticket storage.
    ///
    /// This function should be idempotent,
    /// i.e. calling it multiple times, or calling it on an already-initialized storage should do
    /// nothing.
    ///
    /// Returns the shape the ticket table was found under.
    async fn init_tickets(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<TableShape, SchedulerError<Svc::Error, Sto::Error, MSto::Error>>;

    /// Clear the data from the ticket storage, assuming the tables are already initialized.
    async fn clear_tickets(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<(), SchedulerError<Svc::Error, Sto::Error, MSto::Error>>;

    /// Put the default (fully unresolved) tickets into the ticket storage.
    async fn put_default_tickets(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<(), SchedulerError<Svc::Error, Sto::Error, MSto::Error>>;

    /// Get the status of the tickets.
    async fn get_status(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<(i64, i64, i64), SchedulerError<Svc::Error, Sto::Error, MSto::Error>>;

    /// Run a check on the data consistency between the data storage and the metadata storage.
    /// Return `true` if the data storage holds everything needed to restore.
    async fn check_consistency(
        &self,
        storage: &Sto,
        client: MSto::Client<'_>,
        mode: CheckMode,
    ) -> Result<bool, SchedulerError<Svc::Error, Sto::Error, MSto::Error>>;

    /// Hydrates the scratch in-memory database using the metadata storage.
    async fn hydrate_mem(
        &self,
        src: MSto::Client<'_>,
        dst: MemClient<'_>,
    ) -> Result<(), SchedulerError<Svc::Error, Sto::Error, MSto::Error>>;

    /// Dumps the metadata from scratch in-memory database to the metadata storage.
    ///
    /// Existing data in the metadata storage is discarded.
    async fn dump_mem(
        &self,
        src: MemClient<'_>,
        dst: MSto::Client<'_>,
    ) -> Result<(), SchedulerError<Svc::Error, Sto::Error, MSto::Error>>;

    /// Prepare this task's [`TaskRebuilder`] for the given storage and metadata client by fetching
    /// the necessary data.
    async fn prepare_rebuild(
        &self,
        storage: &Sto,
        progress: SharedProgress,
        client: MSto::Client<'_>,
    ) -> Result<
        Box<dyn TaskRebuilder<Svc, Sto, MSto>>,
        SchedulerError<Svc::Error, Sto::Error, MSto::Error>,
    >;

    /// Start this task's individual scheduler and run it.
    #[allow(clippy::too_many_arguments)]
    fn run_scheduler(
        &self,
        service: Arc<Svc>,
        storage: Arc<Sto>,
        meta_storage: MSto,
        progress: SharedProgress,
        peer_txs: ServicePeerEventSenderMap<Svc>,
        peer_rx: ServicePeerEventReceiver<Svc>,
        ctrl_rx: IndividualControlEventReceiver,
        clean: bool,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>;
}

impl<Svc, Sto, TS, const N: usize> ToMemHandler<Svc, Sto> for SpecWithMetadata<Svc, Sto, TS, N>
where
    Svc: OperonService,
    Sto: OperonStorage,
    TS: TaskSpec<Svc, Sto, MemMetaStorage, Job = Job<N>, Ticket = Ticket<N>>,
{
    fn to_mem(&self) -> Box<dyn TaskHandler<Svc, Sto, MemMetaStorage>> {
        Box::new(SpecWithMetadata::new(self.spec.clone(), self.task_meta))
    }
}

#[async_trait]
impl<Svc, Sto, TS, MSto, const N: usize> TaskHandler<Svc, Sto, MSto>
    for SpecWithMetadata<Svc, Sto, TS, N>
where
    Svc: OperonService,
    Sto: OperonStorage,
    MSto: MetaBackend,
    TS: TaskSpec<Svc, Sto, MSto, Job = Job<N>, Ticket = Ticket<N>>,
    Self: ToMemHandler<Svc, Sto>,
{
    fn task_id(&self) -> &'static str {
        self.task_meta.id
    }

    fn all_upstream_tasks(&self) -> Vec<&'static str> {
        self.spec.all_upstream_tasks()
    }

    fn pool_size(&self) -> usize {
        self.spec.pool_size()
    }

    async fn init_resolution(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<TableShape, SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        let Some(spawn_dim_meta) = self.task_meta.spawn_dim_meta() else {
            return Ok(TableShape::CURRENT);
        };
        let id = spawn_dim_meta.id;
        let resolution = client.resolution(spawn_dim_meta);

        let shape = resolution.init().await?;
        if shape.is_stale {
            tracing::warn!(
                "Dimension `{id}` changed shape, so its resolutions were discarded. \
                 A rebuild will discard progress of all jobs over `{id}`."
            );
        }

        Ok(shape)
    }

    async fn clear_resolution(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<(), SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        if let Some(spawn_dim_meta) = self.task_meta.spawn_dim_meta() {
            client.resolution(spawn_dim_meta).clear().await?;
        }
        Ok(())
    }

    async fn init_tickets(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<TableShape, SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        let id = self.task_meta.id;
        let ticket = client.ticket(self.task_meta);

        let shape = ticket.init().await?;
        if shape.is_stale {
            tracing::warn!(
                "Task `{id}` changed shape, so its tickets were discarded. \
                 A rebuild will discard progress of all `{id}` and downstream jobs."
            );
        }

        Ok(shape)
    }

    async fn clear_tickets(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<(), SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        client.ticket(self.task_meta).clear().await?;
        Ok(())
    }

    async fn put_default_tickets(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<(), SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        let default_ticket = self.spec.default_ticket();
        client.ticket(self.task_meta).put(default_ticket).await?;
        Ok(())
    }

    async fn get_status(
        &self,
        client: MSto::Client<'_>,
    ) -> Result<(i64, i64, i64), SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        let status = client.ticket(self.task_meta).get_status().await?;
        Ok(status)
    }

    async fn check_consistency(
        &self,
        storage: &Sto,
        client: MSto::Client<'_>,
        mode: CheckMode,
    ) -> Result<bool, SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        self.spec.check_consistency(storage, client, mode).await
    }

    async fn hydrate_mem(
        &self,
        src: MSto::Client<'_>,
        dst: MemClient<'_>,
    ) -> Result<(), SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        let tickets = src.ticket(self.task_meta).dump().await?;
        dst.ticket(self.task_meta)
            .hydrate(tickets)
            .await
            .map_err(|e| SchedulerError::MetaStorage(e.during_rebuild()))?;

        if let Some(spawn_dim_meta) = self.task_meta.spawn_dim_meta() {
            let resolutions = src.resolution(spawn_dim_meta).dump().await?;
            dst.resolution(spawn_dim_meta)
                .hydrate(resolutions)
                .await
                .map_err(|e| SchedulerError::MetaStorage(e.during_rebuild()))?;
        }

        Ok(())
    }

    async fn dump_mem(
        &self,
        src: MemClient<'_>,
        dst: MSto::Client<'_>,
    ) -> Result<(), SchedulerError<Svc::Error, Sto::Error, MSto::Error>> {
        let tickets = src
            .ticket(self.task_meta)
            .dump()
            .await
            .map_err(|e| SchedulerError::MetaStorage(e.during_rebuild()))?;
        dst.ticket(self.task_meta).hydrate(tickets).await?;

        if let Some(spawn_dim_meta) = self.task_meta.spawn_dim_meta() {
            let resolutions = src
                .resolution(spawn_dim_meta)
                .dump()
                .await
                .map_err(|e| SchedulerError::MetaStorage(e.during_rebuild()))?;
            dst.resolution(spawn_dim_meta).hydrate(resolutions).await?;
        }

        Ok(())
    }

    async fn prepare_rebuild(
        &self,
        storage: &Sto,
        progress: SharedProgress,
        client: MSto::Client<'_>,
    ) -> Result<
        Box<dyn TaskRebuilder<Svc, Sto, MSto>>,
        SchedulerError<Svc::Error, Sto::Error, MSto::Error>,
    > {
        self.spec.prepare_rebuild(storage, progress, client).await
    }

    #[allow(clippy::too_many_arguments)]
    fn run_scheduler(
        &self,
        service: Arc<Svc>,
        storage: Arc<Sto>,
        meta_storage: MSto,
        progress: SharedProgress,
        peer_txs: ServicePeerEventSenderMap<Svc>,
        peer_rx: ServicePeerEventReceiver<Svc>,
        ctrl_rx: IndividualControlEventReceiver,
        clean: bool,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> {
        let individual_scheduler = IndividualScheduler::new(
            self.clone(),
            service,
            storage,
            meta_storage,
            self.pool_size(),
            progress,
        );
        Box::pin(individual_scheduler.run(peer_txs, peer_rx, ctrl_rx, clean))
    }
}
