use std::hash::{Hash, Hasher};
use std::sync::Arc;

use tokio::sync::OnceCell;
use twox_hash::XxHash3_64;

use crate::meta_storage::psql::error::PsqlResult;
use crate::meta_storage::psql::{
    PsqlClient, PsqlConn, PsqlMetaError, PsqlResolutionQueryBuilder, PsqlTicketQueryBuilder, PsqlTx,
};
use crate::meta_storage::{MetaBackend, MetaStorageError};

/// The Postgres implementation of the metadata backend.
///
/// Holds the connection pools and the schema advisory lock that guard a single Operon run's
/// metadata against concurrent corruption.
#[derive(Debug, Clone)]
pub struct PsqlMetaStorage {
    /// The pool for individual job workers.
    pub worker_pool: deadpool_postgres::Pool,
    /// The pool for the top-level schedulers.
    pub scheduler_pool: deadpool_postgres::Pool,
    /// The optional schema name.
    pub schema: Option<String>,
    pub(super) lock_pool: deadpool_postgres::Pool,
    pub(super) lock: Arc<OnceCell<AdvisoryLock>>,
}

/// The connection holding a schema's advisory lock, and everything needed to check on it later.
#[derive(Debug)]
pub(crate) struct AdvisoryLock {
    pub(super) conn: deadpool_postgres::Object,
    pub(super) schema: String,
    pub(super) key: i64,
}

impl MetaBackend for PsqlMetaStorage {
    type Error = PsqlMetaError;
    type Conn<'a> = PsqlConn<'a>;
    type Tx<'a> = PsqlTx<'a>;
    type Client<'a> = PsqlClient<'a>;
    type Ticket<'a, const N: usize> = PsqlTicketQueryBuilder<'a, N>;
    type Resolution<'a, const N: usize> = PsqlResolutionQueryBuilder<'a, N>;

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
        let _ = self.lock().await?;
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
