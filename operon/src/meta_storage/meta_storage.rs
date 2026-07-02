use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use secrecy::ExposeSecret;
use tokio::sync::OnceCell;

use crate::meta_storage::meta_client::ConnectionWithSchema;
use crate::meta_storage::{MetaStorageError, MetaStorageOptions};

#[derive(Debug, Clone)]
pub struct MetaStorage {
    pub worker_pool: deadpool_postgres::Pool,
    pub scheduler_pool: deadpool_postgres::Pool,
    pub schema: Option<String>,
    lock_pool: deadpool_postgres::Pool,
    lock: Arc<OnceCell<AdvisoryLock>>,
}

/// The connection holding a schema's advisory lock, and everything needed to check on it later.
#[derive(Debug)]
struct AdvisoryLock {
    conn: deadpool_postgres::Object,
    schema: String,
    key: i64,
}

impl MetaStorage {
    pub fn new(options: MetaStorageOptions) -> Result<Self, MetaStorageError> {
        let mk_pool = |size| {
            create_pool(
                options.database_uri.expose_secret(),
                options.keepalives_idle,
                options.keepalives_interval,
                size,
            )
        };

        // The user specifies how many connections we may open to the metadata database.
        // We require at least two, and split the connections into three tiers:
        // 1. One single connection for the advisory lock on the schema.
        // 2. One to five connections for synchronous (individual-)scheduler operations.
        // 3. The remaining connections for spawned workers.
        let remaining = options
            .pool_size
            .checked_sub(1)
            .ok_or(MetaStorageError::PoolSizeTooSmall(options.pool_size))?;
        let lock_pool = mk_pool(1)?;

        let (worker_pool, scheduler_pool) = match remaining {
            0 => return Err(MetaStorageError::PoolSizeTooSmall(options.pool_size)),
            1 => {
                let p = mk_pool(1)?;
                (p.clone(), p)
            }
            n @ ..=5 => (mk_pool(n - 1)?, mk_pool(1)?),
            n => (mk_pool(n - 5)?, mk_pool(5)?),
        };
        let schema = options.schema;

        Ok(MetaStorage {
            worker_pool,
            scheduler_pool,
            schema,
            lock_pool,
            lock: Arc::new(OnceCell::new()),
        })
    }

    pub async fn worker_conn(&self) -> Result<ConnectionWithSchema<'_>, MetaStorageError> {
        let client = self.worker_pool.get().await?;
        let schema = self.schema.as_deref();

        Ok(ConnectionWithSchema::new(client, schema))
    }

    pub async fn scheduler_conn(&self) -> Result<ConnectionWithSchema<'static>, MetaStorageError> {
        let client = self.scheduler_pool.get().await?;
        let schema = self.schema.clone();

        Ok(ConnectionWithSchema::new(client, schema))
    }

    /// Acquires a Postgres session-level advisory lock scoped to this instance's schema,
    /// guarding against a second Operon instance corrupting this run's metadata.
    ///
    /// The lock is tied to the connection that acquires it, held for the lifetime of the
    /// last `Arc` reference to this `MetaStorage` instance.\
    /// Crash safety is provided by Postgres, which releases the lock when the connection closes.
    /// Since that release can also happen silently (see `check_lock`), it isn't the sole guard.
    pub async fn acquire_lock(&self) -> Result<(), MetaStorageError> {
        let conn = self.lock_pool.get().await?;
        let schema = match &self.schema {
            Some(schema) => schema.clone(),
            // Unprefixed queries resolve against whatever schema is first in `search_path`.
            None => conn
                .query_one("SELECT current_schema()", &[])
                .await?
                .get::<_, Option<String>>(0)
                .ok_or(MetaStorageError::Internal(
                    "failed to find default schema; \
                    try setting `meta_storage_schema` explicitly in `OperonOptions`",
                ))?,
        };

        let key = lock_key(&schema);
        let locked: bool = conn
            .query_one("SELECT pg_try_advisory_lock($1)", &[&key])
            .await?
            .get(0);

        if !locked {
            return Err(MetaStorageError::SchemaLocked(schema));
        }

        // `acquire_lock` is only ever called once per real instance (at scheduler
        // startup), so the cell is always empty here.
        let _ = self.lock.set(AdvisoryLock { conn, schema, key });
        Ok(())
    }

    /// Re-checks that the advisory lock acquired by `acquire_lock` is still held, by looking
    /// it up in `pg_locks` rather than trusting the connection to still be alive.
    ///
    /// TCP keepalives (see `MetaStorageOptions`) only catch a peer that has gone completely
    /// unreachable; they don't catch a connection pooler or an `idle_session_timeout` on the
    /// server closing an idle session out from under us, which silently releases the lock
    /// without either side telling us. Since `acquire_lock`'s connection is never used again
    /// after startup, nothing else would ever notice this on its own.
    ///
    /// # Panics
    ///
    /// Panics if called before `acquire_lock` has succeeded.
    pub async fn check_lock(&self) -> Result<(), MetaStorageError> {
        let lock = self
            .lock
            .get()
            .expect("check_lock called before acquire_lock succeeded");

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
            .await?
            .get(0);

        if !held {
            return Err(MetaStorageError::LockLost(lock.schema.clone()));
        }

        Ok(())
    }
}

/// Derives a stable 64-bit key for use with Postgres advisory locks.
///
/// Uses the FNV-1a hash algorithm on the schema name.\
/// `std::hash::Hasher` was not used because its default algorithm is not guaranteed to be stable
/// across builds.
fn lock_key(schema: &str) -> i64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let hash = schema
        .as_bytes()
        .iter()
        .fold(FNV_OFFSET_BASIS, |hash, byte| {
            (hash ^ *byte as u64).wrapping_mul(FNV_PRIME)
        });

    hash as i64
}

fn create_pool(
    uri: &str,
    keepalives_idle: Duration,
    keepalives_interval: Duration,
    pool_size: usize,
) -> Result<deadpool_postgres::Pool, MetaStorageError> {
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
