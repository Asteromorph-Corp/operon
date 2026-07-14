use std::hash::{Hash, Hasher};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use secrecy::ExposeSecret;
use tokio::sync::OnceCell;
use twox_hash::XxHash3_64;

use crate::meta_storage::psql::error::PsqlResult;
use crate::meta_storage::psql::{
    PsqlClient, PsqlConn, PsqlMetaError, PsqlMetaStorageOptions, PsqlResolutionQueryBuilder,
    PsqlTicketQueryBuilder, PsqlTx,
};
use crate::meta_storage::{MetaBackend, MetaStorageError};

/// The Postgres implementation of the metadata backend.
///
/// Holds the connection pools and the schema advisory lock that guard a single Operon run's
/// metadata against concurrent corruption.
#[derive(Debug, Clone)]
pub struct PsqlMetaStorage {
    pub worker_pool: deadpool_postgres::Pool,
    pub scheduler_pool: deadpool_postgres::Pool,
    pub schema: Option<String>,
    lock_pool: deadpool_postgres::Pool,
    lock: Arc<OnceCell<AdvisoryLock>>,
}

/// The connection holding a schema's advisory lock, and everything needed to check on it later.
#[derive(Debug)]
pub(crate) struct AdvisoryLock {
    conn: deadpool_postgres::Object,
    schema: String,
    key: i64,
}

impl MetaBackend for PsqlMetaStorage {
    type Options = PsqlMetaStorageOptions;
    type Error = PsqlMetaError;
    type Conn<'a> = PsqlConn<'a>;
    type Tx<'a> = PsqlTx<'a>;
    type Client<'a> = PsqlClient<'a>;
    type Ticket<'a, const N: usize> = PsqlTicketQueryBuilder<'a, N>;
    type Resolution<'a, const N: usize> = PsqlResolutionQueryBuilder<'a, N>;

    fn new(options: PsqlMetaStorageOptions) -> PsqlResult<Self> {
        let PsqlMetaStorageOptions {
            uri,
            pool_size,
            schema,
            keepalives_idle,
            keepalives_interval,
        } = options;

        let mk_pool = |size| {
            create_pool(
                uri.expose_secret(),
                keepalives_idle,
                keepalives_interval,
                size,
            )
        };

        // The user specifies how many connections we may open to the metadata database.
        // We require at least two, and split the connections into three tiers:
        // 1. One single connection for the advisory lock on the schema.
        // 2. One to five connections for synchronous (individual-)scheduler operations.
        // 3. The remaining connections for spawned workers.
        let (worker_pool, scheduler_pool, lock_pool) = match pool_size {
            0..=1 => return Err(PsqlMetaError::PoolSizeTooSmall(pool_size).into()),
            2 => {
                let p = mk_pool(1)?;
                (p.clone(), p, mk_pool(1)?)
            }
            n @ ..=6 => (mk_pool(n - 2)?, mk_pool(1)?, mk_pool(1)?),
            n => (mk_pool(n - 6)?, mk_pool(5)?, mk_pool(1)?),
        };

        Ok(PsqlMetaStorage {
            worker_pool,
            scheduler_pool,
            schema,
            lock_pool,
            lock: Arc::new(OnceCell::new()),
        })
    }

    async fn worker_conn(&self) -> PsqlResult<PsqlConn<'_>> {
        let client = self.worker_pool.get().await.map_err(PsqlMetaError::from)?;
        let schema = self.schema.as_deref();

        Ok(PsqlConn::new(client, schema))
    }

    async fn scheduler_conn(&self) -> PsqlResult<PsqlConn<'static>> {
        let client = self
            .scheduler_pool
            .get()
            .await
            .map_err(PsqlMetaError::from)?;
        let schema = self.schema.clone();

        Ok(PsqlConn::new(client, schema))
    }

    async fn ensure_lock(&self) -> PsqlResult<()> {
        self.lock().await?;
        Ok(())
    }

    async fn check_lock(&self) -> PsqlResult<()> {
        let lock = self.lock().await?;

        // A single `bigint` advisory lock is recorded in `pg_locks` as its key's upper and
        // lower 32 bits, in `classid`/`objid` respectively, with `objsubid` fixed to 1.
        let classid = (lock.key >> 32) as u32;
        let objid = lock.key as u32;

        let held: bool = lock
            .conn
            .query_one(
                "SELECT EXISTS (
                    SELECT 1 FROM pg_locks
                    WHERE locktype = 'advisory'
                      AND granted
                      AND pid = pg_backend_pid()
                      AND objsubid = 1
                      AND classid = $1
                      AND objid = $2
                )",
                &[&classid, &objid],
            )
            .await
            .map_err(PsqlMetaError::from)?
            .get(0);

        if !held {
            return Err(MetaStorageError::LockLost(lock.schema.clone()));
        }

        Ok(())
    }
}

impl PsqlMetaStorage {
    /// Acquires a Postgres session-level advisory lock scoped to this instance's schema,
    /// guarding against a second Operon instance corrupting this run's metadata.
    ///
    /// The lock is tied to the connection that acquires it, held for the lifetime of the
    /// last `Arc` reference to this `PsqlMetaStorage` instance.\
    /// Crash safety is provided by Postgres, which releases the lock when the connection closes.
    /// Since that release can also happen silently (see `check_lock`), it isn't the sole guard.
    ///
    /// `Scheduler::work` forces this once, at startup; it is lazy and idempotent, so the lock
    /// acquisition path is guaranteed to be reached exactly once per `PsqlMetaStorage` instance.
    async fn lock(&self) -> PsqlResult<&AdvisoryLock> {
        self.lock
            .get_or_try_init(|| async {
                let conn = self.lock_pool.get().await.map_err(PsqlMetaError::from)?;
                let schema = match &self.schema {
                    Some(schema) => schema.clone(),
                    // Unprefixed queries resolve against whatever schema is first in `search_path`.
                    None => conn
                        .query_one("SELECT current_schema()", &[])
                        .await
                        .map_err(PsqlMetaError::from)?
                        .get::<_, Option<String>>(0)
                        .ok_or(MetaStorageError::Internal(
                            "failed to find default schema; \
                            try setting `meta_storage_schema` explicitly in `OperonOptions`",
                        ))?,
                };

                let key = lock_key(&schema);
                let locked: bool = conn
                    .query_one("SELECT pg_try_advisory_lock($1)", &[&key])
                    .await
                    .map_err(PsqlMetaError::from)?
                    .get(0);

                if !locked {
                    return Err(MetaStorageError::SchemaLocked(schema));
                }

                Ok(AdvisoryLock { conn, schema, key })
            })
            .await
    }
}

/// Derives a stable 64-bit key for use with Postgres advisory locks.
///
/// Uses `XxHash3_64` on the schema name.\
/// `std::hash::Hasher`'s default algorithm was not used because it is not guaranteed to be
/// stable across builds;
/// `XxHash3_64` (also used in `utils::sql::hash_metadata`) is used in its place.
fn lock_key(schema: &str) -> i64 {
    let mut hasher = XxHash3_64::new();
    schema.hash(&mut hasher);
    hasher.finish() as i64
}

fn create_pool(
    uri: &str,
    keepalives_idle: Duration,
    keepalives_interval: Duration,
    pool_size: usize,
) -> Result<deadpool_postgres::Pool, PsqlMetaError> {
    // Might want to make these hardcoded config values configurable.
    let mut pg_config = tokio_postgres::Config::from_str(uri)?;
    pg_config
        .keepalives(true)
        .keepalives_idle(keepalives_idle)
        .keepalives_interval(keepalives_interval);
    let manager_config = deadpool_postgres::ManagerConfig {
        recycling_method: deadpool_postgres::RecyclingMethod::Clean,
    };
    let manager =
        deadpool_postgres::Manager::from_config(pg_config, tokio_postgres::NoTls, manager_config);
    let pool = deadpool_postgres::Pool::builder(manager)
        .max_size(pool_size)
        .build()?;
    Ok(pool)
}
