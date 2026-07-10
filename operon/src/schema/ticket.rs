use std::fmt::Debug;
use std::str::FromStr;

use postgres_types::{FromSql, ToSql};

use crate::schema::{Job, OptionCoordinate};

#[derive(Debug, Clone, Copy)]
pub struct Ticket<const N: usize> {
    pub coordinate: [OptionCoordinate; N],
    deps_done: usize,
    deps_quota: usize,
    pub status: TicketStatus,
}

/// A ticket's lifecycle status.
///
/// The `ToSql`/`FromSql` derive maps this onto the Postgres `ticket_status` enum type.
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

    /// Reconstructs a ticket from its raw parts.
    ///
    /// Used by storage backends when materializing a ticket read back from persistence.
    pub(crate) fn from_parts(
        coordinate: [OptionCoordinate; N],
        deps_done: usize,
        deps_quota: usize,
        status: TicketStatus,
    ) -> Self {
        Self {
            coordinate,
            deps_done,
            deps_quota,
            status,
        }
    }

    pub(crate) fn deps_done(&self) -> usize {
        self.deps_done
    }

    pub(crate) fn deps_quota(&self) -> usize {
        self.deps_quota
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
