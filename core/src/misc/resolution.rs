use std::fmt::Debug;

use async_trait::async_trait;

use crate::{
    meta_storage::{MetaClient, MetaStorageError},
    service::OperonService,
};

pub trait Resolution: Debug + Clone + Send + Sync + 'static {
    type PrimaryKey: Copy;

    fn primary_key(&self) -> Self::PrimaryKey;

    fn ub(&self) -> usize;

    fn new(ub: usize, primary_key: Self::PrimaryKey) -> Self;
}

#[async_trait]
pub trait ResolutionSql<Svc: OperonService>: Resolution {
    async fn init_table(client: MetaClient<'_>) -> Result<(), MetaStorageError>;

    async fn clear_table(client: MetaClient<'_>) -> Result<(), MetaStorageError>;

    async fn get(
        client: MetaClient<'_>,
        primary_key: Self::PrimaryKey,
    ) -> Result<Option<Self>, MetaStorageError>;

    async fn put(&self, client: MetaClient<'_>) -> Result<(), MetaStorageError>;
}

impl Resolution for () {
    type PrimaryKey = ();

    fn primary_key(&self) -> Self::PrimaryKey {}

    fn ub(&self) -> usize {
        0
    }

    fn new(_ub: usize, _primary_key: Self::PrimaryKey) -> Self {}
}

pub trait ResolutionEnum: Debug + Clone + Send + Sync + 'static {
    fn primary(resolution: usize) -> Self;
}
