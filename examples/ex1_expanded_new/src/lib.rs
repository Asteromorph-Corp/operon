/// `A`: primary data, repeats on: `i`
#[derive(Debug, Clone, operon::serde::Serialize, operon::serde::Deserialize)]
#[serde(crate = "operon::serde")]
pub struct A(pub String);

/// `B`: derived from: `beta|i (A)`, repeats on: `i`, `j`
#[derive(Debug, Clone, operon::serde::Serialize, operon::serde::Deserialize)]
#[serde(crate = "operon::serde")]
pub struct B(pub A, pub usize);

#[operon::async_trait::async_trait]
pub trait MyOperonService: operon::service::OperonService {
    async fn beta(&self, a: &A) -> Result<Vec<B>, Box<dyn std::error::Error + Send + Sync>>;
}

#[operon::async_trait::async_trait]
pub trait MyOperonStorage: operon::storage::OperonStorage {
    async fn put_a(&self, i: schema::IDim, value: &A) -> Result<(), operon::storage::StorageError>;
    async fn get_a(&self, i: schema::IDim) -> Result<Option<A>, operon::storage::StorageError>;

    async fn put_b(
        &self,
        i: schema::IDim,
        j: schema::JDim,
        value: &B,
    ) -> Result<(), operon::storage::StorageError>;
    async fn get_b(
        &self,
        i: schema::IDim,
        j: schema::JDim,
    ) -> Result<Option<B>, operon::storage::StorageError>;

    // Batch operations (only for the operations that are needed in the OperonService trait).
    async fn put_all_b(
        &self,
        i: schema::IDim,
        values: &[B],
    ) -> Result<(), operon::storage::StorageError> {
        // Default implementation: put each value individually.
        for (j, value) in values.iter().enumerate() {
            self.put_b(i, j, value).await?;
        }
        Ok(())
    }
}

mod queries {
    #[allow(unused_imports)]
    use super::*;
    mod facts {
        #[allow(unused_imports)]
        use super::*;

        pub async fn init_resolution_i(
            client: operon::meta_storage::MetaClient<'_>,
        ) -> Result<(), operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let stmt = format!(
                "CREATE TABLE IF NOT EXISTS {schema_prefix}dimension_i (
                    i_ub BIGINT NOT NULL
                );"
            );
            client.execute(&stmt, &[]).await?;
            Ok(())
        }

        pub async fn clear_resolution_i(
            client: operon::meta_storage::MetaClient<'_>,
        ) -> Result<(), operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let stmt = format!("TRUNCATE TABLE {schema_prefix}dimension_i");
            client.execute(&stmt, &[]).await?;
            Ok(())
        }

        pub async fn get_resolution_i(
            client: operon::meta_storage::MetaClient<'_>,
        ) -> Result<Option<schema::IResolution>, operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let stmt = format!("SELECT i_ub FROM {schema_prefix}resolution");
            let row = client.query_opt(&stmt, &[]).await?;
            Ok(row.map(|row| schema::IResolution(row.get::<_, i64>("i_ub") as usize)))
        }

        pub async fn put_resolution_i(
            client: operon::meta_storage::MetaClient<'_>,
            resolution: &schema::IResolution,
        ) -> Result<(), operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let stmt = format!(
                "INSERT INTO {schema_prefix}dimension_i (i_ub) VALUES ($1) ON CONFLICT DO NOTHING"
            );
            client.execute(&stmt, &[&(resolution.0 as i64)]).await?;
            Ok(())
        }

        pub async fn init_resolution_j(
            client: operon::meta_storage::MetaClient<'_>,
        ) -> Result<(), operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let stmt = format!(
                "CREATE TABLE IF NOT EXISTS {schema_prefix}dimension_j (
                    i BIGINT,
                    j_ub BIGINT NOT NULL,
                    PRIMARY KEY (i)
                );"
            );
            client.execute(&stmt, &[]).await?;
            Ok(())
        }

        pub async fn clear_resolution_j(
            client: operon::meta_storage::MetaClient<'_>,
        ) -> Result<(), operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let stmt = format!("TRUNCATE TABLE {schema_prefix}dimension_i");
            client.execute(&stmt, &[]).await?;
            Ok(())
        }

        pub async fn get_resolution_j(
            client: operon::meta_storage::MetaClient<'_>,
            i: usize,
        ) -> Result<Option<schema::JResolution>, operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let stmt = format!("SELECT j_ub FROM {schema_prefix}dimension_j WHERE i = $1");
            let row = client.query_opt(&stmt, &[&i64::try_from(i)?]).await?;
            Ok(row.map(|row| schema::JResolution(row.get::<_, i64>("j_ub") as usize, i)))
        }

        pub async fn put_resolution_j(
            client: operon::meta_storage::MetaClient<'_>,
            resolution: &schema::JResolution,
        ) -> Result<(), operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let stmt = format!(
                "INSERT INTO {schema_prefix}dimension_j (i, j_ub) VALUES ($1, $2) ON CONFLICT DO NOTHING"
            );
            client
                .execute(&stmt, &[&(resolution.1 as i64), &(resolution.0 as i64)])
                .await?;
            Ok(())
        }

        pub async fn init_resolution_k(
            client: operon::meta_storage::MetaClient<'_>,
        ) -> Result<(), operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let stmt = format!(
                "CREATE TABLE IF NOT EXISTS {schema_prefix}dimension_k (
                    i BIGINT,
                    k_ub BIGINT NOT NULL,
                    PRIMARY KEY (i)
                );"
            );
            client.execute(&stmt, &[]).await?;
            Ok(())
        }

        pub async fn clear_resolution_k(
            client: operon::meta_storage::MetaClient<'_>,
        ) -> Result<(), operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let stmt = format!("TRUNCATE TABLE {schema_prefix}dimension_k");
            client.execute(&stmt, &[]).await?;
            Ok(())
        }

        pub async fn get_resolution_k(
            client: operon::meta_storage::MetaClient<'_>,
            i: usize,
        ) -> Result<Option<schema::KResolution>, operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let stmt = format!("SELECT k_ub FROM {schema_prefix}dimension_k WHERE i = $1");
            let row = client.query_opt(&stmt, &[&(i as i64)]).await?;
            Ok(row.map(|row| schema::KResolution(row.get::<_, i64>("k_ub") as usize, i)))
        }

        pub async fn put_resolution_k(
            client: operon::meta_storage::MetaClient<'_>,
            resolution: &schema::KResolution,
        ) -> Result<(), operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let stmt = format!(
                "INSERT INTO {schema_prefix}dimension_k (i, k_ub) VALUES ($1, $2) ON CONFLICT DO NOTHING"
            );
            client
                .execute(&stmt, &[&(resolution.1 as i64), &(resolution.0 as i64)])
                .await?;
            Ok(())
        }
    }

    mod tickets {
        use super::*;

        pub async fn init_tickets_beta(
            client: operon::meta_storage::MetaClient<'_>,
        ) -> Result<(), operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let ticket_status_type = client.ticket_status_type();
            let create_table = format!(
                "CREATE TABLE IF NOT EXISTS {schema_prefix}ticket_beta (
                    i BIGINT,
                    resolved BOOLEAN NOT NULL,
                    deps_count BIGINT NOT NULL,
                    deps_quota BIGINT,
                    deps_done BOOLEAN NOT NULL,
                    status {ticket_status_type} NOT NULL,
                    PRIMARY KEY (i)
                );"
            ); // TODO: move this to `Ticket` trait and automate using derive macro
            let init_summary = format!(
                "INSERT INTO {schema_prefix}ticket_summary (job_id, waiting, queued, done)
                VALUES ($1, 0, 0, 0)
                ON CONFLICT DO NOTHING;

                CREATE OR REPLACE TRIGGER ticket_beta_summary_ins_trg
                    AFTER INSERT ON {schema_prefix}ticket_beta
                    REFERENCING NEW TABLE AS NEW_TABLE
                    FOR EACH STATEMENT
                    EXECUTE FUNCTION {schema_prefix}trg_ticket_summary('beta');
                    
                CREATE OR REPLACE TRIGGER ticket_beta_summary_upd_trg
                    AFTER UPDATE ON {schema_prefix}ticket_beta
                    REFERENCING
                        NEW TABLE AS NEW_TABLE
                        OLD TABLE AS OLD_TABLE
                    FOR EACH STATEMENT
                    EXECUTE FUNCTION {schema_prefix}trg_ticket_summary('beta');
                    
                CREATE OR REPLACE TRIGGER ticket_beta_summary_del_trg
                    AFTER DELETE ON {schema_prefix}ticket_beta
                    REFERENCING OLD TABLE AS OLD_TABLE
                    FOR EACH STATEMENT
                    EXECUTE FUNCTION {schema_prefix}trg_ticket_summary('beta');

                CREATE OR REPLACE TRIGGER ticket_beta_summary_trunc_trg
                    AFTER TRUNCATE ON {schema_prefix}ticket_beta
                    FOR EACH STATEMENT
                    EXECUTE FUNCTION {schema_prefix}trg_ticket_summary('beta');"
            );
            client.execute(&create_table, &[]).await?;
            client
                .execute(
                    &init_summary,
                    &[&<schema::BetaJob as operon::schema_base::Job>::id()],
                )
                .await?;
            Ok(())
        }

        pub async fn clear_tickets_beta(
            client: operon::meta_storage::MetaClient<'_>,
        ) -> Result<(), operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let stmt = format!("TRUNCATE TABLE {schema_prefix}ticket_beta");
            client.execute(&stmt, &[]).await?;
            Ok(())
        }

        pub async fn put_tickets_beta(
            client: operon::meta_storage::MetaClient<'_>,
            ticket: &schema::BetaTicket,
        ) -> Result<(), operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let stmt = format!(
                "INSERT INTO {schema_prefix}ticket_beta (i, resolved, deps_count, deps_quota, deps_done, status)
                VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT DO NOTHING",
            );
            let params = operon::schema_base::TicketSql::to_sql_insert_params(ticket)?;
            let params = params
                .iter()
                .map(|p| p.as_ref() as &(dyn operon::postgres_types::ToSql + Sync))
                .collect::<Vec<_>>();

            client.execute(&stmt, &params).await?;
            Ok(())
        }

        pub async fn get_all_beta(
            client: operon::meta_storage::MetaClient<'_>,
            status: operon::schema_base::TicketStatus,
        ) -> Result<Vec<schema::BetaTicket>, operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            let stmt = format!("SELECT * FROM {schema_prefix}ticket_beta WHERE status = $1");
            let rows = client.query(&stmt, &[&status]).await?;
            let jobs = rows
                .iter()
                .map(|row| {
                    <schema::BetaTicket as operon::schema_base::TicketSql>::from_sql_row(row)
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(jobs)
        }

        pub async fn explode_beta_i(
            client: operon::meta_storage::MetaClient<'_>,
            resolution: schema::IResolution,
        ) -> Result<Vec<schema::BetaTicket>, operon::meta_storage::MetaStorageError> {
            let schema_prefix = client.schema_prefix();
            // Statement to select all tickets that match the *parent dimensions* in the resolution.
            // (In this case, there are none.)
            let pop_stmt: String = format!(
                "WITH target AS (SELECT * FROM {schema_prefix}ticket_beta),
                popped AS (
                    DELETE FROM {schema_prefix}ticket_beta
                    USING target
                    WHERE ticket_beta.i = target.i
                    RETURNING ticket_beta.*
                )
                SELECT * FROM popped"
            );

            let rows = client.query(&pop_stmt, &[]).await?;
            let tickets = rows
                .iter()
                .map(<schema::BetaTicket as operon::schema_base::TicketSql>::from_sql_row)
                .collect::<Result<Vec<_>, _>>()?;

            if tickets.iter().any(|ticket| ticket.i.is_some()) {
                return Err(operon::meta_storage::MetaStorageError::InvalidResolution(
                    "Called `explode(i)` on `beta`, but `i` was resolved".into(),
                ));
            }
            let new_tickets = tickets
                .iter()
                .flat_map(|ticket| (0..resolution.0).map(|i| ticket.clone().with_i(i)))
                .collect::<Vec<_>>();

            let copy_stmt = format!(
                "COPY {schema_prefix}ticket_beta (i, resolved, deps_count, deps_quota, deps_done, status)
                FROM STDIN WITH (FORMAT csv)"
            );
            let sink = client
                .copy_in::<_, operon::bytes::Bytes>(&copy_stmt)
                .await?;
            let mut sink = Box::pin(sink);
            for ticket in &new_tickets {
                operon::futures::SinkExt::feed(
                    &mut sink,
                    operon::schema_base::TicketSql::to_sql_copy_params(ticket)?.into(),
                )
                .await?;
            }
            operon::futures::SinkExt::close(&mut sink).await?;

            let ready_tickets = new_tickets
                .into_iter()
                .filter(operon::schema_base::Ticket::is_ready)
                .collect::<Vec<_>>();
            Ok(ready_tickets)
        }

        /// Mark a beta ticket as done.
        pub async fn mark_done_beta(
            conn: operon::meta_storage::MetaClient<'_>,
            job: &schema::BetaJob,
        ) -> Result<(), operon::meta_storage::MetaStorageError> {
            let schema_prefix = conn.schema_prefix();
            let stmt =
                format!("UPDATE {schema_prefix}ticket_beta SET status = 'done' WHERE i = $1");
            conn.execute(&stmt, &[&(job.i as i64)]).await?;
            Ok(())
        }
    }

    pub use facts::*;
    pub use tickets::*;
}

mod schema {
    use super::*;

    // generate_operon!();
    mod dimension {
        pub type IDim = usize;
        pub type JDim = usize;
    }

    mod jobs {
        use super::*;

        const BETA_ID: &str = "beta";
        const DELTA_ID: &str = "delta";
        const EPSILON_ID: &str = "epsilon";

        #[derive(Debug, Clone)]
        pub enum MyJobEnum {
            Beta(BetaJob),
        }

        impl operon::schema_base::JobEnum for MyJobEnum {}

        impl From<BetaJob> for MyJobEnum {
            fn from(job: BetaJob) -> Self {
                MyJobEnum::Beta(job)
            }
        }

        #[derive(Debug, Clone, Copy)]
        pub struct BetaJob {
            pub i: usize,
        }

        impl operon::schema_base::Job for BetaJob {
            fn id() -> &'static str {
                BETA_ID
            }

            fn is_descendant_of(other: &str) -> bool {
                other == BETA_ID
            }
        }

        #[operon::async_trait::async_trait]
        impl operon::schema_base::JobSql for BetaJob {
            async fn mark_done(
                &self,
                client: operon::meta_storage::MetaClient<'_>,
            ) -> Result<(), operon::meta_storage::MetaStorageError> {
                queries::mark_done_beta(client, self).await
            }
        }
    }

    mod resolution {
        use super::*;

        #[derive(Debug, Clone)]
        pub enum ResolutionEnum {
            I(IResolution),
            J(JResolution),
            K(KResolution),
        }

        impl operon::schema_base::ResolutionEnum for ResolutionEnum {
            fn primary(resolution: usize) -> Self {
                Self::I(IResolution(resolution))
            }
        }

        impl From<IResolution> for ResolutionEnum {
            fn from(resolution: IResolution) -> Self {
                Self::I(resolution)
            }
        }

        impl From<JResolution> for ResolutionEnum {
            fn from(resolution: JResolution) -> Self {
                Self::J(resolution)
            }
        }

        impl From<KResolution> for ResolutionEnum {
            fn from(resolution: KResolution) -> Self {
                Self::K(resolution)
            }
        }

        #[derive(Debug, Clone, Copy)]
        pub struct IResolution(pub IDim);

        impl operon::schema_base::Resolution for IResolution {
            type PrimaryKey = ();

            #[allow(clippy::unused_unit)]
            fn primary_key(&self) -> Self::PrimaryKey {
                ()
            }

            fn ub(&self) -> usize {
                self.0
            }

            #[allow(unused_variables)]
            fn new(ub: usize, primary_key: Self::PrimaryKey) -> Self {
                IResolution(ub)
            }
        }

        #[operon::async_trait::async_trait]
        impl operon::schema_base::ResolutionSql for IResolution {
            async fn init_table(
                client: operon::meta_storage::MetaClient<'_>,
            ) -> Result<(), operon::meta_storage::MetaStorageError> {
                queries::init_resolution_i(client).await
            }

            async fn clear_table(
                client: operon::meta_storage::MetaClient<'_>,
            ) -> Result<(), operon::meta_storage::MetaStorageError> {
                queries::clear_resolution_i(client).await
            }

            #[allow(unused_variables)]
            async fn get(
                client: operon::meta_storage::MetaClient<'_>,
                primary_key: Self::PrimaryKey,
            ) -> Result<Option<Self>, operon::meta_storage::MetaStorageError> {
                queries::get_resolution_i(client).await
            }

            async fn put(
                &self,
                client: operon::meta_storage::MetaClient<'_>,
            ) -> Result<(), operon::meta_storage::MetaStorageError> {
                queries::put_resolution_i(client, self).await
            }
        }

        #[derive(Debug, Clone, Copy)]
        pub struct JResolution(pub JDim, pub IDim);

        impl operon::schema_base::Resolution for JResolution {
            type PrimaryKey = (IDim,);

            fn primary_key(&self) -> Self::PrimaryKey {
                (self.1,)
            }

            fn ub(&self) -> usize {
                self.0
            }

            #[allow(unused_variables)]
            fn new(ub: usize, primary_key: Self::PrimaryKey) -> Self {
                JResolution(ub, primary_key.0)
            }
        }

        #[operon::async_trait::async_trait]
        impl operon::schema_base::ResolutionSql for JResolution {
            async fn init_table(
                client: operon::meta_storage::MetaClient<'_>,
            ) -> Result<(), operon::meta_storage::MetaStorageError> {
                queries::init_resolution_j(client).await
            }

            async fn clear_table(
                client: operon::meta_storage::MetaClient<'_>,
            ) -> Result<(), operon::meta_storage::MetaStorageError> {
                queries::clear_resolution_j(client).await
            }

            #[allow(unused_variables)]
            async fn get(
                client: operon::meta_storage::MetaClient<'_>,
                primary_key: Self::PrimaryKey,
            ) -> Result<Option<Self>, operon::meta_storage::MetaStorageError> {
                queries::get_resolution_j(client, primary_key.0).await
            }

            async fn put(
                &self,
                client: operon::meta_storage::MetaClient<'_>,
            ) -> Result<(), operon::meta_storage::MetaStorageError> {
                queries::put_resolution_j(client, self).await
            }
        }

        #[derive(Debug, Clone, Copy)]
        pub struct KResolution(pub JDim, pub IDim);

        impl operon::schema_base::Resolution for KResolution {
            type PrimaryKey = IDim;

            fn primary_key(&self) -> Self::PrimaryKey {
                self.1
            }

            fn ub(&self) -> usize {
                self.0
            }

            #[allow(unused_variables)]
            fn new(ub: usize, primary_key: Self::PrimaryKey) -> Self {
                KResolution(ub, primary_key)
            }
        }

        #[operon::async_trait::async_trait]
        impl operon::schema_base::ResolutionSql for KResolution {
            async fn init_table(
                client: operon::meta_storage::MetaClient<'_>,
            ) -> Result<(), operon::meta_storage::MetaStorageError> {
                queries::init_resolution_k(client).await
            }

            async fn clear_table(
                client: operon::meta_storage::MetaClient<'_>,
            ) -> Result<(), operon::meta_storage::MetaStorageError> {
                queries::clear_resolution_k(client).await
            }

            #[allow(unused_variables)]
            async fn get(
                client: operon::meta_storage::MetaClient<'_>,
                primary_key: Self::PrimaryKey,
            ) -> Result<Option<Self>, operon::meta_storage::MetaStorageError> {
                queries::get_resolution_k(client, primary_key).await
            }

            async fn put(
                &self,
                client: operon::meta_storage::MetaClient<'_>,
            ) -> Result<(), operon::meta_storage::MetaStorageError> {
                queries::put_resolution_k(client, self).await
            }
        }
    }

    mod tickets {
        use super::*;

        #[derive(Debug, Clone, Default)]
        pub struct BetaTicket {
            pub i: operon::schema_base::TicketDepCount<IDim>,
            deps_count: usize,
            deps_quota: Option<usize>,
            deps_done: bool,
            status: operon::schema_base::TicketStatus,
        }

        impl BetaTicket {
            pub fn with_i(self, i: IDim) -> Self {
                let mut new = BetaTicket {
                    i: i.into(),
                    ..self
                };

                if operon::schema_base::Ticket::is_ready(&new) {
                    new.status = operon::schema_base::TicketStatus::Queued;
                }

                new
            }
        }

        #[operon::async_trait::async_trait]
        impl operon::schema_base::Ticket for BetaTicket {
            type Job = BetaJob;
            type Resolution = JResolution;

            async fn get_dependency_quota(
                &self,
                _client: operon::meta_storage::MetaClient<'_>,
            ) -> Result<Option<usize>, operon::meta_storage::MetaStorageError> {
                // beta_i dependencies: none
                debug_assert!(false, "Called get_dependency_quota on beta_i");
                Ok(Some(0))
            }

            async fn raise_dependency_count(
                &mut self,
                client: operon::meta_storage::MetaClient<'_>,
            ) -> Result<(), operon::meta_storage::MetaStorageError> {
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
                    .then(|| Some(BetaJob { i: self.i.0? }))
                    .flatten()
            }
        }

        #[operon::async_trait::async_trait]
        impl operon::schema_base::TicketSql for BetaTicket {
            fn to_sql_insert_params(
                &self,
            ) -> Result<
                Vec<Box<dyn operon::postgres_types::ToSql + Send + Sync>>,
                operon::meta_storage::MetaStorageError,
            > {
                let i = self.i.to_sql()?;
                let resolved = operon::schema_base::Ticket::is_resolved(self);
                let deps_count = i64::try_from(self.deps_count)?;
                let deps_quota = self.deps_quota.map(i64::try_from).transpose()?;
                let deps_done = self.deps_done;
                let status = self.status;

                Ok(vec![
                    Box::new(i),
                    Box::new(resolved),
                    Box::new(deps_count),
                    Box::new(deps_quota),
                    Box::new(deps_done),
                    Box::new(status),
                ])
            }

            fn to_sql_copy_params(&self) -> Result<String, operon::meta_storage::MetaStorageError> {
                Ok(format!(
                    "{},{},{},{},{},{}\n",
                    self.i.to_sql()?,
                    operon::schema_base::Ticket::is_resolved(self),
                    self.deps_count,
                    self.deps_quota.map_or(String::new(), |q| q.to_string()),
                    self.deps_done,
                    self.status,
                ))
            }

            fn from_sql_row(
                row: &operon::tokio_postgres::Row,
            ) -> Result<Self, operon::meta_storage::MetaStorageError> {
                let i = operon::schema_base::TicketDepCount::from_sql(row.get("i"))?;
                // let resolved: bool = row.get("resolved");
                let deps_count = usize::try_from(row.get::<_, i64>("deps_count"))?;
                let deps_quota = row
                    .get::<_, Option<i64>>("deps_quota")
                    .map(usize::try_from)
                    .transpose()?;
                let deps_done: bool = row.get("deps_done");
                let status: operon::schema_base::TicketStatus = row.get("status");

                Ok(BetaTicket {
                    i,
                    // resolved,
                    deps_count,
                    deps_quota,
                    deps_done,
                    status,
                })
            }

            async fn init_table(
                client: operon::meta_storage::MetaClient<'_>,
            ) -> Result<(), operon::meta_storage::MetaStorageError> {
                queries::init_tickets_beta(client).await
            }

            async fn clear_table(
                client: operon::meta_storage::MetaClient<'_>,
            ) -> Result<(), operon::meta_storage::MetaStorageError> {
                queries::clear_tickets_beta(client).await
            }

            async fn put(
                &self,
                client: operon::meta_storage::MetaClient<'_>,
            ) -> Result<(), operon::meta_storage::MetaStorageError> {
                queries::put_tickets_beta(client, self).await
            }

            async fn get_all(
                client: operon::meta_storage::MetaClient<'_>,
                status: operon::schema_base::TicketStatus,
            ) -> Result<Vec<Self>, operon::meta_storage::MetaStorageError> {
                queries::get_all_beta(client, status).await
            }

            async fn get_status(
                client: operon::meta_storage::MetaClient<'_>,
            ) -> Result<(i64, i64, i64), operon::meta_storage::MetaStorageError> {
                operon::meta_storage::get_ticket_summary::<BetaJob>(client).await
            }
        }
    }

    pub use dimension::*;
    pub use jobs::*;
    pub use resolution::*;
    pub use tickets::*;
}

mod spec {
    use crate::queries::get_all_beta;

    use super::*;

    #[derive(Debug, Clone, Copy)]
    pub struct AlphaSpec;

    #[operon::async_trait::async_trait]
    impl<Svc, Sto> operon::scheduler::PrimarySpec<Svc, Sto> for AlphaSpec
    where
        Svc: MyOperonService<JobEnum = schema::MyJobEnum, ResolutionEnum = schema::ResolutionEnum>,
        Sto: MyOperonStorage,
    {
        type Resolution = schema::IResolution;

        async fn check_consistency(
            &self,
            storage: &Sto,
            client: operon::meta_storage::MetaClient<'_>,
            primary_ub: usize,
        ) -> Result<bool, operon::scheduler::SchedulerError> {
            // Pull the primary resolution from the metadata storage..
            let Some(schema::IResolution(i_ub)) =
                <schema::IResolution as operon::schema_base::ResolutionSql>::get(client, ())
                    .await?
            else {
                // This is technically unreachable, because we check this same value
                // in `check_recovery_state`.
                operon::log::info!("No primary resolution found in the metadata storage.");
                return Ok(false);
            };
            // ...and check if the data storage holds all the data for it.
            for i in 0..i_ub.max(primary_ub) {
                if storage.get_a(i).await?.is_none() {
                    operon::log::info!("Data storage does not hold `A_{i}`.");
                    return Ok(false);
                }
            }
            // Additionally check if the primary resolution agrees with the given upper bound.
            if i_ub != primary_ub {
                operon::log::warn!(
                    "Previous run's upper bound `{i_ub}` is different from the current run's upper bound `{primary_ub}`. \n\
                    If you overwrote the primary data, consider running `run --fresh` to overwrite the existing data, \
                    otherwise the resulting data may be inconsistent. \n\
                    If you want to keep the existing data, and intendedly set the upper bound to `{primary_ub}`, \
                    you may ignore this warning."
                );
            }

            return Ok(true);
        }
    }

    #[derive(Debug)]
    pub struct BetaRebuilder(Vec<(schema::BetaJob, schema::JResolution)>);

    #[derive(Debug)]
    pub struct BetaPeerTxs {
        pub to_delta: operon::scheduler::PeerEventSender<schema::MyJobEnum, schema::ResolutionEnum>,
        pub to_epsilon:
            operon::scheduler::PeerEventSender<schema::MyJobEnum, schema::ResolutionEnum>,
    }

    #[operon::async_trait::async_trait]
    impl operon::scheduler::PeerEventSenders<schema::MyJobEnum, schema::ResolutionEnum>
        for BetaPeerTxs
    {
        fn gather_from(
            mut senders: operon::scheduler::PeerEventSenderMap<
                schema::MyJobEnum,
                schema::ResolutionEnum,
            >,
        ) -> Self {
            BetaPeerTxs {
                to_delta: senders
                    // .remove(<schema::DeltaJob as operon::schema_base::Job>::id())
                    .remove("delta")
                    .unwrap_or_else(|| panic!("No delta sender found")),
                to_epsilon: senders
                    // .remove(<schema::EpsilonJob as operon::schema_base::Job>::id())
                    .remove("epsilon")
                    .unwrap_or_else(|| panic!("No epsilon sender found")),
            }
        }

        fn downgrade_all(&mut self) {
            self.to_delta.downgrade();
            self.to_epsilon.downgrade();
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub struct BetaSpec;

    #[operon::async_trait::async_trait]
    impl<Svc, Sto> operon::scheduler::JobSpec<Svc, Sto> for BetaSpec
    where
        Svc: MyOperonService<JobEnum = schema::MyJobEnum, ResolutionEnum = schema::ResolutionEnum>,
        Sto: MyOperonStorage,
    {
        type Job = schema::BetaJob;
        type Resolution = schema::JResolution;
        type Ticket = schema::BetaTicket;
        type PeerEventSenders = BetaPeerTxs;

        async fn check_consistency(
            &self,
            storage: &Sto,
            client: operon::meta_storage::MetaClient<'_>,
        ) -> Result<bool, operon::scheduler::SchedulerError> {
            // Pull the "done" beta jobs from the metadata storage...
            let Some(jobs) = <schema::BetaTicket as operon::schema_base::TicketSql>::get_all(
                client,
                operon::schema_base::TicketStatus::Done,
            )
            .await?
            .iter()
            .map(operon::schema_base::Ticket::resolve)
            .collect::<Option<Vec<_>>>() else {
                operon::log::info!("Some `beta` tickets are corrupt in the metadata storage.");
                return Ok(false);
            };
            // ...and map them with the dimensions they spawned...
            let mut tags = vec![];
            for job in jobs {
                let Some(res) = queries::get_resolution_j(client, job.i).await? else {
                    operon::log::info!(
                        "No `j` resolution found for `beta_{}` in the metadata storage.",
                        job.i
                    );
                    return Ok(false);
                };
                for j in 0..(res.0) {
                    tags.push((job.i, j));
                }
            }
            // ...and check if the data storage holds all the data for them.
            for (i, j) in tags {
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
            client: operon::meta_storage::MetaClient<'_>,
        ) -> Result<Box<dyn operon::scheduler::JobRebuilder>, operon::scheduler::SchedulerError>
        {
            let tickets =
                queries::get_all_beta(client, operon::schema_base::TicketStatus::Done).await?;
            let successes = operon::futures::future::try_join_all(tickets.into_iter().map(
                |ticket| async move {
                    let job = operon::schema_base::Ticket::resolve(&ticket).ok_or_else(|| {
                        operon::scheduler::SchedulerError::Other(
                            "Failed to resolve a beta ticket".into(),
                        )
                    })?;
                    let resolution =
                        queries::get_resolution_j(client, job.i)
                            .await?
                            .ok_or_else(|| {
                                operon::scheduler::SchedulerError::Other(format!(
                                    "No resolution found for `J_{}`",
                                    job.i
                                ))
                            })?;

                    Ok::<_, operon::scheduler::SchedulerError>((job, resolution))
                },
            ))
            .await?;
            Ok(Box::new(BetaRebuilder(successes)))
        }

        async fn run_job(
            &self,
            service: &Svc,
            storage: &Sto,
            _client: operon::meta_storage::MetaClient<'_>,
            job: &Self::Job,
        ) -> Result<Self::Resolution, operon::scheduler::SchedulerError> {
            let a = storage
                .get_a(job.i)
                .await?
                .ok_or(operon::storage::StorageError::NotFound)?;
            let b_j = service
                .beta(&a)
                .await
                .map_err(operon::scheduler::SchedulerError::UserError)?;
            storage.put_all_b(job.i, &b_j).await?;

            Ok(schema::JResolution(b_j.len(), job.i))
        }

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

        #[allow(unused_variables, clippy::match_single_binding)]
        async fn on_receive_job(
            &self,
            client: operon::meta_storage::MetaClient<'_>,
            job: schema::MyJobEnum,
        ) -> Result<Vec<Self::Ticket>, operon::scheduler::SchedulerError> {
            // Beta jobs don't have dependencies.
            match job {
                _ => Err(operon::scheduler::SchedulerError::InvalidPeerEventReceived(
                    "job",
                    <Self::Job as operon::schema_base::Job>::id(),
                )),
            }
        }

        async fn on_receive_resolution(
            &self,
            client: operon::meta_storage::MetaClient<'_>,
            resolution: schema::ResolutionEnum,
        ) -> Result<Vec<schema::BetaTicket>, operon::scheduler::SchedulerError> {
            match resolution {
                schema::ResolutionEnum::I(resolution) => {
                    let tickets = queries::explode_beta_i(client, resolution).await?;
                    Ok(tickets)
                }
                _ => Err(operon::scheduler::SchedulerError::InvalidPeerEventReceived(
                    "resolution",
                    <Self::Job as operon::schema_base::Job>::id(),
                )),
            }
        }
    }

    #[operon::async_trait::async_trait]
    impl operon::scheduler::JobRebuilder for BetaRebuilder {
        async fn explode(
            &self,
            client: operon::meta_storage::MetaClient<'_>,
            primary_ub: usize,
        ) -> Result<(), operon::scheduler::SchedulerError> {
            queries::explode_beta_i(client, schema::IResolution(primary_ub)).await?;
            Ok(())
        }

        async fn rebuild(
            &self,
            client: operon::meta_storage::MetaClient<'_>,
        ) -> Result<(), operon::scheduler::SchedulerError> {
            for (job, resolution) in &self.0 {
                // What we would do at a job success:
                queries::put_resolution_j(client, resolution).await?;
                queries::mark_done_beta(client, job).await?;

                // // Roll them out to its dependencies:
                // queries::explode_delta(client, &resolution).await?;
                // queries::raise_dep_delta(
                //     client,
                //     &masked_dimension::I::One(i),
                //     &masked_dimension::J::All(masked_dimension::I::One(i)),
                //     &masked_dimension::K::All(masked_dimension::I::One(i)),
                // )
                // .await?;
                // queries::raise_dep_epsilon(
                //     client,
                //     &masked_dimension::I::One(i),
                //     &masked_dimension::K::All(masked_dimension::I::One(i)),
                // )
                // .await?;
            }

            Ok(())
        }
    }
}
