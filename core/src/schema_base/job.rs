use std::fmt::Debug;

use async_trait::async_trait;

use crate::meta_storage::{MetaClient, MetaStorageError};

pub trait Job: Debug + Clone + Send + Sync + 'static {}

#[async_trait]
pub trait JobSql: Job {
    async fn mark_done(&self, client: MetaClient<'_>) -> Result<(), MetaStorageError>;
}

pub trait JobEnum: Debug + Clone + Send + Sync + 'static {}
