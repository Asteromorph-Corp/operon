use crate::meta_storage::psql::PsqlMetaStorage;
use crate::meta_storage::{MetaBackendOptions, MetaConn, MetaStorageError, MetaStorageOptions};

/// The backend-agnostic handle over the concrete metadata store.
#[derive(Debug, Clone)]
pub enum MetaStorage {
    Psql(PsqlMetaStorage),
}

impl MetaStorage {
    pub fn new(options: MetaStorageOptions) -> Result<Self, MetaStorageError> {
        let MetaStorageOptions {
            backend,
            pool_size,
            keepalives_idle,
            keepalives_interval,
            schema,
        } = options;

        // The only backend right now.
        // When further backends are added they get their own arm here,
        // constructed from their variant's parameters.
        match backend {
            MetaBackendOptions::Psql(uri) => Ok(MetaStorage::Psql(PsqlMetaStorage::new(
                &uri,
                pool_size,
                keepalives_idle,
                keepalives_interval,
                schema,
            )?)),
        }
    }

    /// Checks out a connection for a spawned worker.
    pub async fn worker_conn(&self) -> Result<MetaConn<'_>, MetaStorageError> {
        match self {
            MetaStorage::Psql(storage) => Ok(MetaConn::Psql(storage.worker_conn().await?)),
        }
    }

    /// Checks out a connection for synchronous (individual-)scheduler operations.
    pub async fn scheduler_conn(&self) -> Result<MetaConn<'static>, MetaStorageError> {
        match self {
            MetaStorage::Psql(storage) => Ok(MetaConn::Psql(storage.scheduler_conn().await?)),
        }
    }

    /// Acquires the exclusive lock guarding this run's metadata against a second Operon instance.
    ///
    /// Lazy and idempotent; forced once at scheduler startup. For backends without a notion of a
    /// cross-instance lock this is a no-op.
    pub(crate) async fn ensure_lock(&self) -> Result<(), MetaStorageError> {
        match self {
            MetaStorage::Psql(storage) => {
                storage.ensure_lock().await?;
                Ok(())
            }
        }
    }

    /// Re-checks that the lock acquired by [`ensure_lock`](Self::ensure_lock) is still held.
    pub(crate) async fn check_lock(&self) -> Result<(), MetaStorageError> {
        match self {
            MetaStorage::Psql(storage) => storage.check_lock().await,
        }
    }
}
