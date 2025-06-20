use async_trait::async_trait;

use crate::meta_storage::{MetaStorage, MetaStorageError};

/// `Promoter` trait for promoting a service to a storage object. TODO: fix this comment
#[async_trait]
pub trait Promoter<Sto, Svc> {
    async fn init_facts(&self, meta_storage: &MetaStorage) -> Result<(), MetaStorageError>;
    async fn init_tickets(&self, meta_storage: &MetaStorage) -> Result<(), MetaStorageError>;
}
