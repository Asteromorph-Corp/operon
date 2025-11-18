use std::fmt::Debug;
use std::num::TryFromIntError;

use async_trait::async_trait;

use crate::meta_storage::{MetaClient, MetaStorageError};
use crate::utils::SqlParams;

pub struct ResolutionStruct<const N: usize> {
    pub primary_key: [usize; N],
    pub ub: usize,
}

impl<const N: usize> ResolutionStruct<N> {
    pub fn new(ub: usize, primary_key: [usize; N]) -> Self {
        Self { primary_key, ub }
    }

    pub fn as_insert_params(&self) -> Result<SqlParams, TryFromIntError> {
        SqlParams::from_usize(self.primary_key.into_iter().chain([self.ub]))
    }
}

pub trait Resolution: Debug + Clone + Send + Sync + 'static {
    type PrimaryKey: Copy;

    fn primary_key(&self) -> Self::PrimaryKey;

    fn ub(&self) -> usize;

    fn new(ub: usize, primary_key: Self::PrimaryKey) -> Self;
}

#[async_trait]
pub trait ResolutionSql: Resolution {
    async fn init_table(client: MetaClient<'_>) -> Result<(), MetaStorageError>;

    async fn clear_table(client: MetaClient<'_>) -> Result<(), MetaStorageError>;

    async fn get(
        client: MetaClient<'_>,
        primary_key: Self::PrimaryKey,
    ) -> Result<Option<Self>, MetaStorageError>;

    async fn put(&self, client: MetaClient<'_>) -> Result<(), MetaStorageError>;
}

pub trait ResolutionEnum: Debug + Clone + Send + Sync + 'static {}

impl Resolution for () {
    type PrimaryKey = ();

    fn primary_key(&self) -> Self::PrimaryKey {}

    fn ub(&self) -> usize {
        0
    }

    fn new(_ub: usize, _primary_key: Self::PrimaryKey) -> Self {}
}

#[async_trait]
impl ResolutionSql for () {
    async fn init_table(_client: MetaClient<'_>) -> Result<(), MetaStorageError> {
        Ok(())
    }

    async fn clear_table(_client: MetaClient<'_>) -> Result<(), MetaStorageError> {
        Ok(())
    }

    async fn get(
        _client: MetaClient<'_>,
        _primary_key: Self::PrimaryKey,
    ) -> Result<Option<Self>, MetaStorageError> {
        Ok(Some(()))
    }

    async fn put(&self, _client: MetaClient<'_>) -> Result<(), MetaStorageError> {
        Ok(())
    }
}
