use std::fmt::Debug;
use std::num::TryFromIntError;

use async_trait::async_trait;
use tokio_postgres::Row;

use crate::meta_storage::{MetaClient, MetaStorageError};
use crate::schema::{Job, JobMetadata, OptionCoordinate, TicketStatus};
use crate::utils::{SqlParams, box_sql};

#[derive(Debug, Clone, Copy)]
pub struct Ticket<const N: usize> {
    pub coordinate: [OptionCoordinate; N],
    deps_done: usize,
    deps_quota: usize,
    pub status: TicketStatus,
}

impl<const N: usize> Ticket<N> {
    pub const fn new(quota: usize) -> Self {
        let status = if quota == 0 {
            TicketStatus::Queued
        } else {
            TicketStatus::Waiting
        };

        Self {
            coordinate: [OptionCoordinate::none(); N],
            deps_done: 0usize,
            deps_quota: quota,
            status,
        }
    }

    pub const fn update_status(mut self) -> Self {
        if self.is_ready() {
            self.status = TicketStatus::Queued;
        }
        self
    }

    pub const fn raise_deps_quota(mut self, explosion_ub: usize) -> Self {
        self.deps_quota = self.deps_quota + explosion_ub - 1;
        self.update_status()
    }

    pub const fn with_coordinate<const IDX: usize>(mut self, coordinate: usize) -> Self {
        const { assert!(IDX < N) }
        self.coordinate[IDX] = OptionCoordinate::some(coordinate);
        self
    }

    pub const fn is_ready(&self) -> bool {
        self.deps_done >= self.deps_quota
    }

    pub const fn resolve(&self) -> Option<Job<N>> {
        if !self.is_ready() {
            return None;
        }

        let mut coordinate = [0usize; N];
        let mut i = 0;

        while i < N {
            let Some(src) = self.coordinate[i].0 else {
                return None;
            };
            coordinate[i] = src;
            i += 1;
        }

        Some(Job { coordinate })
    }

    pub fn as_sql_params(&self) -> Result<SqlParams, TryFromIntError> {
        let params = self
            .coordinate
            .iter()
            .map(|c| c.as_sql_param().map(box_sql))
            .chain([
                i64::try_from(self.deps_done).map(box_sql),
                i64::try_from(self.deps_quota).map(box_sql),
                Ok(box_sql(self.status)),
            ])
            .collect::<Result<Vec<_>, _>>()?;

        Ok(SqlParams::new(params))
    }

    pub fn to_copy_string(&self) -> Result<String, TryFromIntError> {
        Ok(self.as_sql_params()?.to_copy_string())
    }

    pub fn from_sql_row(meta: JobMetadata<N>, row: &Row) -> Result<Self, TryFromIntError> {
        let mut coordinate = [OptionCoordinate::none(); N];
        let mut i = 0;

        while i < N {
            let c = OptionCoordinate::from_sql_value(row.get(meta.dims[i]))?;
            coordinate[i] = c;
            i += 1;
        }

        let deps_done = usize::try_from(row.get::<_, i64>("deps_done"))?;
        let deps_quota = usize::try_from(row.get::<_, i64>("deps_quota"))?;
        let status: TicketStatus = row.get("status");

        Ok(Self {
            coordinate,
            deps_done,
            deps_quota,
            status,
        })
    }
}

pub trait TicketLike: Debug + Clone + Copy + Send + Sync + 'static {}

impl<const N: usize> TicketLike for Ticket<N> {}

#[async_trait]
pub trait TicketSql: Sized {
    /// Initialize the ticket storage in the metadata storage.
    async fn init_table(client: MetaClient<'_>) -> Result<(), MetaStorageError>;

    /// Clear the ticket storage in the metadata storage.
    async fn clear_table(client: MetaClient<'_>) -> Result<(), MetaStorageError>;

    /// Put the ticket into the metadata storage.
    async fn put(&self, client: MetaClient<'_>) -> Result<(), MetaStorageError>;

    /// Get all tickets that is `status = 'done'`.
    async fn get_all(
        client: MetaClient<'_>,
        status: TicketStatus,
    ) -> Result<Vec<Self>, MetaStorageError>;

    /// Get the status of the tickets.
    async fn get_status(client: MetaClient<'_>) -> Result<(i64, i64, i64), MetaStorageError>;
}
