use operon::{
    async_trait::async_trait,
    futures::{SinkExt, future::try_join_all},
    meta_storage::{MetaClient, MetaStorage, MetaStorageError},
    misc::{Job, OptionExt, Resolution, Ticket, TicketStatus},
    operon::OperonError,
    scheduler::{
        IndividualSchedule, IndividualSchedulerOps, InternalEvent, JobManager, JobRebuilder,
        SchedulerError,
    },
    service::OperonService,
    storage::{OperonStorage, StorageError},
};

// generate_operon!();
mod dimension {
    pub type I = usize;
    pub type J = usize;
}

mod entity {
    use operon::serde::{Deserialize, Serialize};

    /// `A`: primary data, repeats on: `i`
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(crate = "operon::serde")]
    pub struct A(pub String);

    /// `B`: derived from: `beta|i (A)`, repeats on: `i`, `j`
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(crate = "operon::serde")]
    pub struct B(pub A, pub usize);
}

#[async_trait]
pub trait MyOperonService: OperonService {
    async fn beta(
        &self,
        a: &entity::A,
    ) -> Result<Vec<entity::B>, Box<dyn std::error::Error + Send + Sync>>;
}

#[async_trait]
pub trait MyOperonStorage: OperonStorage {
    async fn put_a(&self, i: dimension::I, value: &entity::A) -> Result<(), StorageError>;
    async fn get_a(&self, i: dimension::I) -> Result<Option<entity::A>, StorageError>;

    async fn put_b(
        &self,
        i: dimension::I,
        j: dimension::J,
        value: &entity::B,
    ) -> Result<(), StorageError>;
    async fn get_b(
        &self,
        i: dimension::I,
        j: dimension::J,
    ) -> Result<Option<entity::B>, StorageError>;

    // Batch operations (only for the operations that are needed in the OperonService trait).
    async fn put_all_b(&self, i: dimension::I, values: &[entity::B]) -> Result<(), StorageError> {
        // Default implementation: put each value individually.
        for (j, value) in values.iter().enumerate() {
            self.put_b(i, j, value).await?;
        }
        Ok(())
    }
}

#[async_trait]
pub trait MyMetaStorage: MetaStorage {
    async fn get_resolution_i(
        &self,
        client: MetaClient<'_>,
    ) -> Result<Option<IResolution>, MetaStorageError> {
        let schema_prefix = client.schema_prefix();
        let stmt = format!("SELECT i_ub FROM {schema_prefix}resolution");
        let row = client.query_opt(&stmt, &[]).await?;
        Ok(row.map(|row| IResolution(row.get::<_, i64>("i_ub") as usize)))
    }

    async fn put_resolution_i(
        &self,
        client: MetaClient<'_>,
        resolution: &IResolution,
    ) -> Result<(), MetaStorageError> {
        let schema_prefix = client.schema_prefix();
        let stmt = format!(
            "INSERT INTO {schema_prefix}dimension_i (i_ub) VALUES ($1) ON CONFLICT DO NOTHING"
        );
        client.execute(&stmt, &[&(resolution.0 as i64)]).await?;
        Ok(())
    }

    async fn get_resolution_j(
        &self,
        client: MetaClient<'_>,
        i: usize,
    ) -> Result<Option<JResolution>, MetaStorageError> {
        let schema_prefix = client.schema_prefix();
        let stmt = format!("SELECT j_ub FROM {schema_prefix}resolution WHERE i = $1");
        let row = client.query_opt(&stmt, &[&(i as i64)]).await?;
        Ok(row.map(|row| JResolution(row.get::<_, i64>("j_ub") as usize, i)))
    }

    async fn put_resolution_j(
        &self,
        client: MetaClient<'_>,
        resolution: &JResolution,
    ) -> Result<(), MetaStorageError> {
        let schema_prefix = client.schema_prefix();
        let stmt = format!(
            "INSERT INTO {schema_prefix}dimension_j (i, j_ub) VALUES ($1, $2) ON CONFLICT DO NOTHING"
        );
        client
            .execute(&stmt, &[&(resolution.1 as i64), &(resolution.0 as i64)])
            .await?;
        Ok(())
    }

    async fn explode_beta(
        &self,
        client: MetaClient<'_>,
        resolution: &ResolutionEnum,
    ) -> Result<Vec<BetaTicket>, MetaStorageError> {
        let schema_prefix = client.schema_prefix();
        match resolution {
            ResolutionEnum::I(i) => {
                // Statement to select all tickets that match the *parent dimensions* in the resolution.
                // (In this case, there are none.)
                let stmt: String = format!(
                    "WITH target AS (SELECT * FROM {schema_prefix}ticket_beta),
                    popped AS (
                        DELETE FROM {schema_prefix}ticket_beta
                        USING target
                        WHERE ticket_beta.i = target.i
                        RETURNING ticket_beta.*
                    )
                    SELECT * FROM popped"
                );
                let rows = client.query(&stmt, &[]).await?;
                let tickets = rows
                    .into_iter()
                    .map(|row| BetaTicket::from_sql_row(&row))
                    .collect::<Result<Vec<_>, _>>()?;
                let stmt = format!(
                            "COPY {schema_prefix}ticket_beta (i, resolved, deps_count, deps_quota, deps_done, status)
                            FROM STDIN WITH (FORMAT csv)"
                        );
                let sink: ::tokio_postgres::CopyInSink<operon::bytes::Bytes> =
                    client.copy_in(&stmt).await?;
                let mut sink = Box::pin(sink);
                let ready_tickets = {
                    // We can feed to the sink directly,
                    // since we don't need the connection inside the loop
                    // in the explode operation.
                    let mut ready_tickets = Vec::new();
                    for ticket in tickets {
                        let just_exploded = ticket.explode_i(*i)?;
                        for mut exploded_ticket in just_exploded {
                            if exploded_ticket.is_ready() {
                                exploded_ticket.status = TicketStatus::Queued;
                            }
                            sink.feed(exploded_ticket.to_sql_copy_params()?.into())
                                .await?;
                            if exploded_ticket.is_ready() {
                                ready_tickets.push(exploded_ticket);
                            }
                        }
                    }
                    ready_tickets
                };
                sink.close().await;
                Ok(ready_tickets)
            }
            _ => Err(MetaStorageError::InvalidResolution(
                "Called irrelevant explode on beta_i".to_string(),
            )),
        }
    }

    /// Mark a beta ticket as done.
    async fn mark_done_beta(
        &self,
        conn: MetaClient<'_>,
        job: &BetaJob,
    ) -> Result<(), SchedulerError> {
        let schema_prefix = conn.schema_prefix();
        let stmt = format!("UPDATE {schema_prefix}ticket_beta SET status = 'done' WHERE i = $1");
        conn.execute(&stmt, &[&(job.i as i64)]).await?;
        Ok(())
    }
}

pub enum JobEnum {
    Beta(BetaJob),
}

#[derive(Debug, Clone, Copy)]
pub struct IResolution(pub dimension::I);

pub enum ResolutionEnum {
    I(IResolution),
    J(JResolution),
}

impl From<BetaJob> for JobEnum {
    fn from(job: BetaJob) -> Self {
        JobEnum::Beta(job)
    }
}

impl From<JResolution> for ResolutionEnum {
    fn from(resolution: JResolution) -> Self {
        ResolutionEnum::J(resolution)
    }
}

pub struct UserStorage;

pub struct UserService;

pub struct UserMetaStorage;

impl MetaStorage for UserMetaStorage {}

impl MyMetaStorage for UserMetaStorage {}

// generate_schedules!(MyStorage, MyService);
const BETA_ID: &str = "beta";

#[derive(Debug, Clone, Default)]
pub struct BetaTicket {
    i: Option<dimension::I>,
    deps_count: usize,
    deps_quota: Option<usize>,
    deps_done: bool,
    pub status: TicketStatus,
}

#[derive(Debug, Clone, Copy)]
pub struct BetaJob {
    pub i: usize,
}

impl Job for BetaJob {}

#[derive(Debug, Clone, Copy)]
pub struct JResolution(pub dimension::J, pub dimension::I);

impl Resolution for JResolution {}

impl BetaTicket {
    fn explode_i(self, resolution: IResolution) -> Result<Vec<Self>, MetaStorageError> {
        if self.is_resolved() {
            return Err(MetaStorageError::InvalidResolution(
                "Called explode on a fully resolved beta_i".into(),
            ));
        }

        if !self.i.is_none() {
            return Err(MetaStorageError::InvalidResolution(
                "Called explode(i) on beta, but i was resolved".to_string(),
            ));
        }

        let out_tickets = (0..resolution.0)
            .map(|i| BetaTicket {
                i: Some(i),
                ..self.clone()
            })
            .collect();
        Ok(out_tickets)
    }
}

#[async_trait]
impl Ticket for BetaTicket {
    type Job = BetaJob;
    type Resolution = JResolution;

    async fn get_dependency_quota(
        &self,
        _client: MetaClient<'_>,
    ) -> Result<Option<usize>, MetaStorageError> {
        // beta_i dependencies: none
        debug_assert!(false, "Called get_dependency_quota on beta_i");
        Ok(Some(0))
    }

    async fn raise_dependency_count(
        &mut self,
        client: MetaClient<'_>,
    ) -> Result<(), MetaStorageError> {
        debug_assert!(false, "Called raise_dependency_count on beta_i");
        self.deps_count += 1;
        if self.deps_quota.is_none() {
            self.deps_quota = self.get_dependency_quota(client).await?;
        }
        self.deps_done = match self.deps_quota {
            Some(quota) => self.deps_count >= quota,
            None => false,
        };
        Ok(())
    }

    fn is_ready(&self) -> bool {
        self.is_resolved() && self.deps_done
    }

    fn is_resolved(&self) -> bool {
        self.i.is_some()
    }

    fn resolve(&self) -> Option<BetaJob> {
        self.is_ready()
            .then(|| Some(BetaJob { i: self.i? }))
            .flatten()
    }

    fn to_sql_insert_params(&self) -> Result<String, MetaStorageError> {
        Ok(format!(
            "({},{},{},{},{},'{}')",
            self.i.to_sql()?, // TODO: remove unwrap
            self.is_resolved(),
            self.deps_count,
            match self.deps_quota {
                Some(quota) => quota.to_string(),
                None => "NULL".to_string(),
            },
            self.deps_done,
            self.status
        ))
    }

    fn to_sql_copy_params(&self) -> Result<String, MetaStorageError> {
        Ok(format!(
            "{},{},{},{},{},{}\n",
            self.i.to_sql()?, // TODO: remove unwrap
            self.is_resolved(),
            self.deps_count,
            match self.deps_quota {
                Some(quota) => quota.to_string(),
                None => String::new(),
            },
            self.deps_done,
            self.status
        ))
    }

    fn from_sql_row(row: &tokio_postgres::Row) -> Result<Self, MetaStorageError> {
        let i: i64 = row.get("i");
        // let resolved: bool = row.get("resolved");
        let deps_count: i64 = row.get("deps_count");
        let deps_quota: Option<i64> = row.get("deps_quota");
        let deps_done: bool = row.get("deps_done");
        let status: TicketStatus = row.get("status");

        let i = match i {
            -1 => None,
            _ => Some(usize::try_from(i)?),
        };
        let deps_quota: Option<usize> = deps_quota.map(|q| q as usize);
        Ok(BetaTicket {
            i,
            // resolved,
            deps_count: deps_count as usize,
            deps_quota,
            deps_done,
            status,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BetaManager;

#[derive(Debug)]
pub struct BetaRebuilder(BetaManager, Vec<(BetaJob, JResolution)>);

impl BetaManager {
    async fn get_all_done(&self, conn: MetaClient<'_>) -> Result<Vec<BetaTicket>, SchedulerError> {
        let schema_prefix = conn.schema_prefix();
        let stmt = format!("SELECT * FROM {schema_prefix}ticket_beta WHERE status = 'done'");
        let rows = conn.query(&stmt, &[]).await?;
        let jobs = rows
            .iter()
            .map(|row| BetaTicket::from_sql_row(row))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(jobs)
    }
}

#[async_trait]
impl<Svc, Sto> IndividualSchedule<Svc, Sto, UserMetaStorage> for BetaManager
where
    Svc: MyOperonService,
    Sto: MyOperonStorage,
{
    fn id(&self) -> &'static str {
        BETA_ID
    }

    async fn check_consistency(
        &self,
        storage: &Sto,
        meta_storage: &UserMetaStorage,
        _primary_ub: usize,
    ) -> Result<bool, SchedulerError> {
        let conn: operon::meta_storage::ConnectionWithSchema<'_> = meta_storage.conn().await?;
        // Pull the "done" beta jobs from the metadata storage...
        let Some(beta_jobs) = self
            .get_all_done(conn.as_client())
            .await?
            .iter()
            .map(|t| t.resolve().and_then(|job| Some((job.i,))))
            .collect::<Option<Vec<_>>>()
        else {
            operon::log::info!("Some `beta` tickets are corrupt in the metadata storage.");
            return Ok(false);
        };
        // ...and map them with the dimensions they spawned...
        let mut b_tags = vec![];
        for (i,) in beta_jobs {
            let Some(JResolution(j_ub, _)) =
                meta_storage.get_resolution_j(conn.as_client(), i).await?
            else {
                operon::log::info!(
                    "No `j` resolution found for `beta_{i}` in the metadata storage."
                );
                return Ok(false);
            };
            for j in 0..j_ub {
                b_tags.push((i, j));
            }
        }
        // ...and check if the data storage holds all the data for them.
        for (i, j) in b_tags {
            if storage.get_b(i, j).await?.is_none() {
                operon::log::info!("Data storage does not hold `B_{i},{j}`.");
                return Ok(false);
            }
        }

        Ok(true)
    }

    async fn prepare_rebuild(
        &self,
        _storage: &Sto,
        meta_storage: &UserMetaStorage,
        tx: MetaClient<'_>,
    ) -> Result<Box<dyn JobRebuilder>, SchedulerError> {
        let beta_tickets = self.get_all_done(tx).await?;
        let beta_successes = try_join_all(beta_tickets.into_iter().map(|ticket| async move {
            let job = ticket
                .resolve()
                .ok_or_else(|| SchedulerError::Other("Failed to resolve a beta ticket".into()))?;
            let resolution = meta_storage
                .get_resolution_j(tx, job.i)
                .await?
                .ok_or_else(|| {
                    SchedulerError::Other(format!("No resolution found for `J_{}`", job.i).into())
                })?;

            Ok::<_, SchedulerError>((job, resolution))
        }))
        .await?;
        Ok(Box::new(BetaRebuilder(self.clone(), beta_successes)))
    }
}

#[async_trait]
impl<Svc, Sto, MSto> JobManager<Svc, Sto, MSto> for BetaManager
where
    Sto: MyOperonStorage,
    Svc: MyOperonService,
{
    type Job = BetaJob;
    type Resolution = JResolution;
    type Ticket = BetaTicket;
    type PeerEventSenders = ();

    fn job_type() -> &'static str {
        BETA_ID
    }

    fn is_descendant_of(job_type: &str) -> bool {
        job_type == BETA_ID
    }

    async fn run_job(
        &self,
        service: &Svc,
        storage: &Sto,
        _conn: MetaClient<'_>,
        job: &BetaJob,
    ) -> Result<JResolution, SchedulerError> {
        let a = storage.get_a(job.i).await?.ok_or(StorageError::NotFound)?;
        let b_j = service.beta(&a).await.map_err(SchedulerError::UserError)?;
        let j_resolution = b_j.len();
        storage.put_all_b(job.i, &b_j).await?;

        Ok(JResolution(j_resolution, job.i))
    }

    /// Mark a beta ticket as done.
    async fn mark_done(
        &self,
        meta_storage: &MSto,
        conn: MetaClient<'_>,
        job: &BetaJob,
    ) -> Result<(), SchedulerError> {
        let schema_prefix = conn.schema_prefix();
        let stmt = format!("UPDATE {schema_prefix}ticket_beta SET status = 'done' WHERE i = $1");
        conn.execute(&stmt, &[&(job.i as i64)]).await?;
        Ok(())
    }

    async fn put_resolution(
        &self,
        conn: MetaClient<'_>,
        schema_prefix: &str,
        resolution: &JResolution,
    ) -> Result<(), SchedulerError> {
        let stmt = format!(
            "INSERT INTO {schema_prefix}resolution_j (i, j) VALUES ($1, $2) ON CONFLICT DO NOTHING"
        );
        conn.execute(&stmt, &[&(resolution.1 as i64), &(resolution.0 as i64)])
            .await?;
        Ok(())
    }
}

#[async_trait]
impl JobRebuilder<UserMetaStorage> for BetaRebuilder {
    async fn explode(
        &self,
        meta_storage: &UserMetaStorage,
        client: MetaClient<'_>,
        primary_ub: usize,
    ) -> Result<(), SchedulerError> {
        meta_storage
            .explode_beta(client, &ResolutionEnum::I(IResolution(primary_ub)))
            .await?;
        Ok(())
    }

    async fn rebuild(
        &self,
        meta_storage: &UserMetaStorage,
        client: MetaClient<'_>,
    ) -> Result<(), SchedulerError> {
        for (job, resolution) in &self.1 {
            // What we would do at a job success:
            meta_storage.put_resolution_j(client, resolution).await?;
            meta_storage.mark_done_beta(client, job).await?;
            meta_storage.explode_delta(client, resolution).await?;

            // Roll them out to its dependencies:
            tickets_psql::explode_delta(conn, &resolution).await?;
            tickets_psql::raise_dep_delta(
                conn,
                &masked_dimension::I::One(i),
                &masked_dimension::J::All(masked_dimension::I::One(i)),
                &masked_dimension::K::All(masked_dimension::I::One(i)),
            )
            .await?;
            tickets_psql::raise_dep_epsilon(
                conn,
                &masked_dimension::I::One(i),
                &masked_dimension::K::All(masked_dimension::I::One(i)),
            )
            .await?;
        }

        Ok(())
    }
}
