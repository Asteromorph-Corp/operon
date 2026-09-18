//! Helpers shared by the metadata backend test modules.

use crate::meta_storage::psql::{PsqlMetaStorage, PsqlMetaStorageOptions};
use crate::schema::{DimensionMetadata, Resolution, TaskMetadata, Ticket};

/// The Postgres backend over the given schema, or `None` when no database is configured.
pub(super) fn psql_backend(schema: &str) -> Option<PsqlMetaStorage> {
    let uri = std::env::var("POSTGRES_URI").ok()?;
    let options = PsqlMetaStorageOptions::new(uri).with_schema(schema);
    Some(options.build().expect("build the Postgres backend"))
}

/// A ticket projected onto its comparable parts.
///
/// `Ticket` is neither `PartialEq` nor `Ord`, and a backend returns tickets in an unspecified
/// order, so observations are projected and sorted before they are compared.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct TicketView {
    coordinate: Vec<Option<usize>>,
    deps_done: usize,
    deps_quota: usize,
    status: String,
}

impl TicketView {
    pub(super) fn of<const N: usize>(ticket: &Ticket<N>) -> Self {
        Self {
            coordinate: ticket.coordinate.iter().map(|coord| coord.0).collect(),
            deps_done: ticket.deps_done(),
            deps_quota: ticket.deps_quota(),
            status: ticket.status.to_string(),
        }
    }
}

/// Projects and sorts a set of tickets into a comparable observation.
pub(super) fn views<const N: usize>(tickets: &[Ticket<N>]) -> Vec<TicketView> {
    let mut views = tickets.iter().map(TicketView::of).collect::<Vec<_>>();
    views.sort();
    views
}

/// Projects and sorts a set of resolutions into a comparable observation.
pub(super) fn resolution_views<const N: usize>(
    resolutions: &[Resolution<N>],
) -> Vec<(Vec<usize>, usize)> {
    let mut views = resolutions
        .iter()
        .map(|res| (res.coordinate.to_vec(), res.ub))
        .collect::<Vec<_>>();
    views.sort();
    views
}

/// A task with no dimensions, spawning the dimension `i`.
pub(super) fn task_alpha() -> TaskMetadata<0> {
    TaskMetadata {
        id: "alpha",
        dims: [],
        spawn_dim: Some("i"),
        priority: &[],
    }
}

/// A task over `i`, downstream of `alpha`.
pub(super) fn task_beta() -> TaskMetadata<1> {
    TaskMetadata {
        id: "beta",
        dims: ["i"],
        spawn_dim: None,
        priority: &[],
    }
}

/// A task over `i`, downstream of `beta`, used to pin `raise_deps_done` to one coordinate.
pub(super) fn task_gamma() -> TaskMetadata<1> {
    TaskMetadata {
        id: "gamma",
        dims: ["i"],
        spawn_dim: None,
        priority: &[],
    }
}

/// A task over both `i` and `j`, whose upstreams pin one of the two.
///
/// Its upstreams pin a proper subset of its dimensions, which is the query shape a single-dimension
/// task cannot produce.
pub(super) fn task_delta() -> TaskMetadata<2> {
    TaskMetadata {
        id: "delta",
        dims: ["i", "j"],
        spawn_dim: None,
        priority: &[],
    }
}

/// An upstream of `delta` over `j` alone.
pub(super) fn task_over_j() -> TaskMetadata<1> {
    TaskMetadata {
        id: "over_j",
        dims: ["j"],
        spawn_dim: None,
        priority: &[],
    }
}

/// The dimension `alpha` spawns.
pub(super) fn dim_i() -> DimensionMetadata<0> {
    DimensionMetadata { id: "i", deps: [] }
}
