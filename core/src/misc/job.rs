use std::fmt::Debug;

use async_trait::async_trait;

use crate::meta_storage::{MetaClient, MetaStorageError};

pub trait Job: Debug + Clone + Send + Sync + 'static {
    fn id() -> &'static str;

    /// Check if this job type is a descendant of the given job type.
    fn is_descendant_of(other: &str) -> bool;
}

#[async_trait]
pub trait JobSql: Job {
    async fn mark_done(&self, client: MetaClient<'_>) -> Result<(), MetaStorageError>;
}

pub trait JobEnum: Debug + Clone + Send + Sync + 'static {}
