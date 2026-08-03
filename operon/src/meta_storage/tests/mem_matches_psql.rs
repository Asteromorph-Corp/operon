//! Differential tests pinning the in-memory backend's semantics to the Postgres backend's.
//!
//! Both backends run an identical sequence of operations and every step's return value and
//! resulting state is compared. This is what keeps the two implementations of `raise_deps_done`,
//! `raise_deps_quota`, `explode` and the ticket summary from drifting apart.
//!
//! These tests need a Postgres to compare against and are skipped when `POSTGRES_URI` is unset.
//! They run against a schema of their own, which they clear before each sequence.

use crate::meta_storage::mem::MemMetaStorage;
use crate::meta_storage::tests::utils::psql_backend;
use crate::meta_storage::{
    MetaBackend, MetaClientApi, MetaConnApi, MetaResolutionApi, MetaTicketApi,
};
use crate::schema::{DimensionMetadata, Job, Resolution, TaskMetadata, Ticket, TicketStatus};

/// The schema the differential sequences own outright.
const SCHEMA: &str = "operon_differential";

/// A task with no dimensions, spawning the dimension `i`.
fn job_alpha() -> TaskMetadata<0> {
    TaskMetadata {
        id: "alpha",
        dims: [],
        spawn_dim: Some("i"),
        priority: &[],
    }
}

/// A task over `i`, downstream of `alpha`.
fn job_beta() -> TaskMetadata<1> {
    TaskMetadata {
        id: "beta",
        dims: ["i"],
        spawn_dim: None,
        priority: &[],
    }
}

/// A task over `i`, downstream of `beta`, used to pin `raise_deps_done` to one coordinate.
fn job_gamma() -> TaskMetadata<1> {
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
/// job cannot produce.
fn job_delta() -> TaskMetadata<2> {
    TaskMetadata {
        id: "delta",
        dims: ["i", "j"],
        spawn_dim: None,
        priority: &[],
    }
}

/// An upstream of `delta` over `j` alone.
fn job_over_j() -> TaskMetadata<1> {
    TaskMetadata {
        id: "over_j",
        dims: ["j"],
        spawn_dim: None,
        priority: &[],
    }
}

/// The dimension `alpha` spawns.
fn dim_i() -> DimensionMetadata<0> {
    DimensionMetadata { id: "i", deps: [] }
}

/// A ticket projected onto its comparable parts.
///
/// `Ticket` is neither `PartialEq` nor `Ord`, and both backends return tickets in an unspecified
/// order, so observations are projected and sorted before they are compared.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct TicketView {
    coordinate: Vec<Option<usize>>,
    deps_done: usize,
    deps_quota: usize,
    status: String,
}

impl TicketView {
    fn of<const N: usize>(ticket: &Ticket<N>) -> Self {
        Self {
            coordinate: ticket.coordinate.iter().map(|coord| coord.0).collect(),
            deps_done: ticket.deps_done(),
            deps_quota: ticket.deps_quota(),
            status: ticket.status.to_string(),
        }
    }
}

/// Projects and sorts a set of tickets into a comparable observation.
fn views<const N: usize>(tickets: &[Ticket<N>]) -> Vec<TicketView> {
    let mut views = tickets.iter().map(TicketView::of).collect::<Vec<_>>();
    views.sort();
    views
}

/// What a single step of a sequence observed.
#[derive(Debug, PartialEq, Eq)]
enum Observed {
    Tickets(Vec<TicketView>),
    Status((i64, i64, i64)),
    Resolution(Option<(Vec<usize>, usize)>),
}

/// One step of a sequence: what it did, and what it observed.
type Step = (&'static str, Observed);

/// Runs the sequence against `backend`, recording every step's return value and resulting state.
///
/// Kept generic over the backend so both run byte-identical logic.
async fn exercise<MSto: MetaBackend>(backend: &MSto) -> Vec<Step> {
    let conn = backend.scheduler_conn().await.expect("scheduler conn");
    let client = conn.as_client();
    let mut log = Vec::new();

    // Initialize in the same order the scheduler does.
    client.init_schema().await.expect("init_schema");
    client
        .init_dimension_hash()
        .await
        .expect("init_dimension_hash");
    client.init_ticket_hash().await.expect("init_ticket_hash");
    client
        .resolution(dim_i())
        .init()
        .await
        .expect("resolution init");
    client
        .init_ticket_summary()
        .await
        .expect("init_ticket_summary");
    client
        .init_ticket_status_type()
        .await
        .expect("init_ticket_status_type");
    client.ticket(job_alpha()).init().await.expect("alpha init");
    client.ticket(job_beta()).init().await.expect("beta init");
    client.ticket(job_gamma()).init().await.expect("gamma init");
    client.ticket(job_delta()).init().await.expect("delta init");
    client.init_footprint().await.expect("init_footprint");

    // Start from a known state, so a reused database matches a fresh store.
    client
        .resolution(dim_i())
        .clear()
        .await
        .expect("resolution clear");
    client
        .ticket(job_alpha())
        .clear()
        .await
        .expect("alpha clear");
    client.ticket(job_beta()).clear().await.expect("beta clear");
    client
        .ticket(job_gamma())
        .clear()
        .await
        .expect("gamma clear");
    client
        .ticket(job_delta())
        .clear()
        .await
        .expect("delta clear");
    client.clear_footprint().await.expect("clear_footprint");

    // A zero-quota ticket is ready on arrival; beta's default waits on one dependency.
    client
        .ticket(job_alpha())
        .put(Ticket::new(0))
        .await
        .expect("alpha put");
    client
        .ticket(job_beta())
        .put(Ticket::new(1))
        .await
        .expect("beta put");
    log.push((
        "alpha status after put",
        Observed::Status(
            client
                .ticket(job_alpha())
                .get_status()
                .await
                .expect("status"),
        ),
    ));
    log.push((
        "beta status after put",
        Observed::Status(
            client
                .ticket(job_beta())
                .get_status()
                .await
                .expect("status"),
        ),
    ));

    // Putting the same coordinate again must not overwrite the ticket already there.
    client
        .ticket(job_beta())
        .put(Ticket::new(99))
        .await
        .expect("beta duplicate put");
    log.push((
        "beta waiting after duplicate put",
        Observed::Tickets(views(
            &client
                .ticket(job_beta())
                .get_all(TicketStatus::Waiting)
                .await
                .expect("get_all"),
        )),
    ));

    // Resolve `i` to three values.
    client
        .resolution(dim_i())
        .put(Resolution {
            coordinate: [],
            ub: 3,
        })
        .await
        .expect("resolution put");
    let resolution = client
        .resolution(dim_i())
        .get([])
        .await
        .expect("resolution get");
    log.push((
        "resolution i",
        Observed::Resolution(resolution.map(|res| (res.coordinate.to_vec(), res.ub))),
    ));

    // Exploding beta along `i` pops its unresolved ticket and fans it out over the resolution.
    let popped = client
        .ticket(job_beta())
        .explode::<0, 0>(
            dim_i(),
            Resolution {
                coordinate: [],
                ub: 3,
            },
        )
        .await
        .expect("beta explode");
    log.push(("beta explode returned", Observed::Tickets(views(&popped))));
    log.push((
        "beta status after explode",
        Observed::Status(
            client
                .ticket(job_beta())
                .get_status()
                .await
                .expect("status"),
        ),
    ));
    log.push((
        "beta waiting after explode",
        Observed::Tickets(views(
            &client
                .ticket(job_beta())
                .get_all(TicketStatus::Waiting)
                .await
                .expect("get_all"),
        )),
    ));

    // Raising the quota holds the tickets back until a second dependency lands.
    let raised = client
        .ticket(job_beta())
        .raise_deps_quota::<0>(job_alpha(), Ticket::new(0), &[], 2)
        .await
        .expect("beta raise_deps_quota");
    log.push((
        "beta raise_deps_quota returned",
        Observed::Tickets(views(&raised)),
    ));
    log.push((
        "beta waiting after raise_deps_quota",
        Observed::Tickets(views(
            &client
                .ticket(job_beta())
                .get_all(TicketStatus::Waiting)
                .await
                .expect("get_all"),
        )),
    ));

    // One dependency is short of the raised quota; the second meets it.
    let first = client
        .ticket(job_beta())
        .raise_deps_done::<0>(job_alpha(), Job { coordinate: [] }, &[])
        .await
        .expect("beta raise_deps_done");
    log.push((
        "beta raise_deps_done returned (short of quota)",
        Observed::Tickets(views(&first)),
    ));
    let second = client
        .ticket(job_beta())
        .raise_deps_done::<0>(job_alpha(), Job { coordinate: [] }, &[])
        .await
        .expect("beta raise_deps_done");
    log.push((
        "beta raise_deps_done returned (quota met)",
        Observed::Tickets(views(&second)),
    ));
    log.push((
        "beta status after raise_deps_done",
        Observed::Status(
            client
                .ticket(job_beta())
                .get_status()
                .await
                .expect("status"),
        ),
    ));

    // Marking one coordinate done must leave its siblings alone.
    client
        .ticket(job_beta())
        .mark_done(Job { coordinate: [0] })
        .await
        .expect("beta mark_done");
    log.push((
        "beta status after mark_done",
        Observed::Status(
            client
                .ticket(job_beta())
                .get_status()
                .await
                .expect("status"),
        ),
    ));
    log.push((
        "beta done after mark_done",
        Observed::Tickets(views(
            &client
                .ticket(job_beta())
                .get_all(TicketStatus::Done)
                .await
                .expect("get_all"),
        )),
    ));

    // Gamma pins `raise_deps_done` to a single upstream coordinate.
    for coord in 0..3 {
        client
            .ticket(job_gamma())
            .put(Ticket::new(1).with_coordinate::<0>(coord))
            .await
            .expect("gamma put");
    }
    let pinned = client
        .ticket(job_gamma())
        .raise_deps_done::<1>(job_beta(), Job { coordinate: [0] }, &[])
        .await
        .expect("gamma raise_deps_done");
    log.push((
        "gamma raise_deps_done returned (pinned to i = 0)",
        Observed::Tickets(views(&pinned)),
    ));
    log.push((
        "gamma status after pinned raise_deps_done",
        Observed::Status(
            client
                .ticket(job_gamma())
                .get_status()
                .await
                .expect("status"),
        ),
    ));
    log.push((
        "gamma waiting after pinned raise_deps_done",
        Observed::Tickets(views(
            &client
                .ticket(job_gamma())
                .get_all(TicketStatus::Waiting)
                .await
                .expect("get_all"),
        )),
    ));

    // Delta's upstreams each pin one of its two dimensions, leaving the other free.
    for i in 0..2 {
        for j in 0..2 {
            client
                .ticket(job_delta())
                .put(
                    Ticket::new(2)
                        .with_coordinate::<0>(i)
                        .with_coordinate::<1>(j),
                )
                .await
                .expect("delta put");
        }
    }
    let over_i = client
        .ticket(job_delta())
        .raise_deps_done::<1>(job_gamma(), Job { coordinate: [0] }, &[])
        .await
        .expect("delta raise_deps_done over i");
    log.push((
        "delta raise_deps_done returned (pinned to i = 0, j free)",
        Observed::Tickets(views(&over_i)),
    ));
    log.push((
        "delta waiting after raise_deps_done over i",
        Observed::Tickets(views(
            &client
                .ticket(job_delta())
                .get_all(TicketStatus::Waiting)
                .await
                .expect("get_all"),
        )),
    ));

    let over_j = client
        .ticket(job_delta())
        .raise_deps_done::<1>(job_over_j(), Job { coordinate: [0] }, &[])
        .await
        .expect("delta raise_deps_done over j");
    log.push((
        "delta raise_deps_done returned (pinned to j = 0, i free)",
        Observed::Tickets(views(&over_j)),
    ));
    log.push((
        "delta status after both pinned raises",
        Observed::Status(
            client
                .ticket(job_delta())
                .get_status()
                .await
                .expect("status"),
        ),
    ));
    log.push((
        "delta queued after both pinned raises",
        Observed::Tickets(views(
            &client
                .ticket(job_delta())
                .get_all(TicketStatus::Queued)
                .await
                .expect("get_all"),
        )),
    ));

    // A job with no dimensions marks its single ticket done without a coordinate to pin on.
    client
        .ticket(job_alpha())
        .mark_done(Job { coordinate: [] })
        .await
        .expect("alpha mark_done");
    log.push((
        "alpha status after mark_done",
        Observed::Status(
            client
                .ticket(job_alpha())
                .get_status()
                .await
                .expect("status"),
        ),
    ));

    log
}

#[tokio::test]
async fn mem_matches_psql() {
    let Some(psql) = psql_backend(SCHEMA) else {
        eprintln!("skipping: POSTGRES_URI is not set, so there is no Postgres to compare against");
        return;
    };

    let from_psql = exercise(&psql).await;
    let from_mem = exercise(&MemMetaStorage::default()).await;

    // Compare step by step, so a drift names the operation that caused it.
    for (psql_step, mem_step) in from_psql.iter().zip(&from_mem) {
        pretty_assertions::assert_eq!(psql_step, mem_step, "backends drifted at: {}", psql_step.0);
    }
    pretty_assertions::assert_eq!(from_psql.len(), from_mem.len());
}
