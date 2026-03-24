use std::fmt::Debug;
use std::num::TryFromIntError;
use std::str::FromStr;

use postgres_types::{FromSql, ToSql};
use tokio_postgres::Row;

use crate::schema::{Job, JobMetadata, OptionCoordinate};
use crate::utils::{SqlParams, box_sql};

#[derive(Debug, Clone, Copy)]
pub struct Ticket<const N: usize> {
    pub coordinate: [OptionCoordinate; N],
    deps_done: usize,
    deps_quota: usize,
    pub status: TicketStatus,
}

#[derive(Debug, Clone, Default, Copy, PartialEq, Eq, ToSql, FromSql)]
#[postgres(name = "ticket_status")]
pub enum TicketStatus {
    #[default]
    #[postgres(name = "waiting")]
    Waiting,
    #[postgres(name = "queued")]
    Queued,
    #[postgres(name = "done")]
    Done,
}

pub trait TicketLike: Debug + Clone + Copy + Send + Sync + 'static {}

impl<const N: usize> TicketLike for Ticket<N> {}

pub trait TicketEnum: Debug + Clone + Send + Sync + 'static {}

#[derive(Debug, Clone)]
pub struct TicketExplosion<T: TicketEnum> {
    /// The ticket providing the coordinate of explosion.
    pub ticket: T,
    /// The dimension of explosion.
    pub dim: &'static str,
    /// The ub of the explosion resolution.
    pub ub: usize,
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

impl ::std::fmt::Display for TicketStatus {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match self {
            TicketStatus::Waiting => write!(f, "waiting"),
            TicketStatus::Queued => write!(f, "queued"),
            TicketStatus::Done => write!(f, "done"),
        }
    }
}

impl FromStr for TicketStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "waiting" => Ok(TicketStatus::Waiting),
            "queued" => Ok(TicketStatus::Queued),
            "done" => Ok(TicketStatus::Done),
            _ => Err(format!("Invalid ticket status: {s}")),
        }
    }
}
