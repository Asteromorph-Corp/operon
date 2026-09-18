//! Tests for the bulk transfer a staged rebuild is carried by.
//!
//! A rebuild against a store that is not already in memory replays into a scratch in-memory store
//! and dumps the result back, so `dump`/`load` have to carry a table over whole: every ticket with
//! its dependency counters and status, every resolution with its bound, and — on the Postgres
//! side — the ticket summary the counters are read back through.
//!
//! The Postgres leg is skipped when `POSTGRES_URI` is unset. It runs against a schema of its own,
//! which it clears before each transfer.

use crate::meta_storage::mem::MemMetaStorage;
use crate::meta_storage::psql::PsqlMetaStorage;
use crate::meta_storage::tests::utils::{
    TicketView, dim_i, psql_backend, resolution_views, task_alpha, task_beta, views,
};
use crate::meta_storage::{
    MetaBackend, MetaClientApi, MetaConnApi, MetaResolutionApi, MetaTicketApi,
};
use crate::schema::{Job, Resolution, Ticket};

/// The schema the transfers own outright.
const SCHEMA: &str = "operon_transfer";

/// Prepares every table this module writes to, and empties them.
async fn init<MSto: MetaBackend>(client: MSto::Client<'_>) {
    client.init_schema().await.expect("init_schema");
    client
        .init_dimension_hash()
        .await
        .expect("init_dimension_hash");
    client.init_ticket_hash().await.expect("init_ticket_hash");
    let _ = client
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
    let _ = client
        .ticket(task_alpha())
        .init()
        .await
        .expect("alpha init");
    let _ = client.ticket(task_beta()).init().await.expect("beta init");

    client
        .resolution(dim_i())
        .clear()
        .await
        .expect("resolution clear");
    client
        .ticket(task_alpha())
        .clear()
        .await
        .expect("alpha clear");
    client
        .ticket(task_beta())
        .clear()
        .await
        .expect("beta clear");
}

/// Fills the tables with a spread of statuses, coordinates, and dependency counters.
///
/// The tickets are put through `put` rather than `load`, so what a transfer carries over is state
/// the ordinary write path produced.
async fn populate<MSto: MetaBackend>(client: MSto::Client<'_>) {
    // Done, without a coordinate to pin on.
    client
        .ticket(task_alpha())
        .put(Ticket::new(0))
        .await
        .expect("alpha put");
    client
        .ticket(task_alpha())
        .mark_done(Job { coordinate: [] })
        .await
        .expect("alpha mark_done");

    // One ticket per status, each carrying different counters.
    client
        .ticket(task_beta())
        .put(Ticket::new(0).with_coordinate::<0>(0))
        .await
        .expect("beta put queued");
    client
        .ticket(task_beta())
        .put(Ticket::new(3).with_coordinate::<0>(1))
        .await
        .expect("beta put waiting");
    client
        .ticket(task_beta())
        .put(Ticket::new(1).with_coordinate::<0>(2))
        .await
        .expect("beta put");
    let _tickets = client
        .ticket(task_beta())
        .raise_deps_done::<0>(task_alpha(), Job { coordinate: [] }, &[])
        .await
        .expect("beta raise_deps_done");
    client
        .ticket(task_beta())
        .mark_done(Job { coordinate: [2] })
        .await
        .expect("beta mark_done");

    client
        .resolution(dim_i())
        .put(Resolution {
            coordinate: [],
            ub: 3,
        })
        .await
        .expect("resolution put");
}

/// What a store holds, projected for comparison.
#[derive(Debug, PartialEq, Eq)]
struct Snapshot {
    alpha: Vec<TicketView>,
    beta: Vec<TicketView>,
    resolutions: Vec<(Vec<usize>, usize)>,
    alpha_status: (i64, i64, i64),
    beta_status: (i64, i64, i64),
}

async fn snapshot<MSto: MetaBackend>(client: MSto::Client<'_>) -> Snapshot {
    Snapshot {
        alpha: views(
            &client
                .ticket(task_alpha())
                .dump()
                .await
                .expect("alpha dump"),
        ),
        beta: views(&client.ticket(task_beta()).dump().await.expect("beta dump")),
        resolutions: resolution_views(
            &client
                .resolution(dim_i())
                .dump()
                .await
                .expect("resolution dump"),
        ),
        alpha_status: client
            .ticket(task_alpha())
            .get_status()
            .await
            .expect("alpha status"),
        beta_status: client
            .ticket(task_beta())
            .get_status()
            .await
            .expect("beta status"),
    }
}

/// Copies every table of `src` into `dst`, as a staged rebuild's two transfers do.
async fn transfer<Src: MetaBackend, Dst: MetaBackend>(src: Src::Client<'_>, dst: Dst::Client<'_>) {
    let tickets = src.ticket(task_alpha()).dump().await.expect("alpha dump");
    dst.ticket(task_alpha())
        .hydrate(tickets)
        .await
        .expect("alpha load");

    let tickets = src.ticket(task_beta()).dump().await.expect("beta dump");
    dst.ticket(task_beta())
        .hydrate(tickets)
        .await
        .expect("beta load");

    let resolutions = src
        .resolution(dim_i())
        .dump()
        .await
        .expect("resolution dump");
    dst.resolution(dim_i())
        .hydrate(resolutions)
        .await
        .expect("resolution load");
}

/// A dump loaded into a second in-memory store leaves that store holding the same metadata.
#[tokio::test]
async fn mem_round_trips_through_mem() {
    let source = MemMetaStorage::default();
    let source_conn = source.scheduler_conn().await.expect("scheduler conn");
    let source_client = source_conn.as_client();
    init::<MemMetaStorage>(source_client).await;
    populate::<MemMetaStorage>(source_client).await;

    let target = MemMetaStorage::default();
    let target_conn = target.scheduler_conn().await.expect("scheduler conn");
    let target_client = target_conn.as_client();
    init::<MemMetaStorage>(target_client).await;

    transfer::<MemMetaStorage, MemMetaStorage>(source_client, target_client).await;

    pretty_assertions::assert_eq!(
        snapshot::<MemMetaStorage>(source_client).await,
        snapshot::<MemMetaStorage>(target_client).await
    );
}

/// The transfers a staged rebuild is built on: Postgres out to a scratch store, and back again.
///
/// The metadata has to survive both legs unchanged, ticket summary included — the summary is
/// maintained by triggers, so this is what pins the `COPY`-based load to keeping them accurate.
#[tokio::test]
async fn psql_round_trips_through_mem() {
    let Some(psql) = psql_backend(SCHEMA) else {
        eprintln!("skipping: POSTGRES_URI is not set, so there is no Postgres to transfer with");
        return;
    };

    let psql_conn = psql.scheduler_conn().await.expect("scheduler conn");
    let psql_client = psql_conn.as_client();
    init::<PsqlMetaStorage>(psql_client).await;
    populate::<PsqlMetaStorage>(psql_client).await;
    let before = snapshot::<PsqlMetaStorage>(psql_client).await;

    // Out to the scratch store the rebuild would replay against.
    let scratch = MemMetaStorage::default();
    let scratch_conn = scratch.scheduler_conn().await.expect("scheduler conn");
    let scratch_client = scratch_conn.as_client();
    init::<MemMetaStorage>(scratch_client).await;
    transfer::<PsqlMetaStorage, MemMetaStorage>(psql_client, scratch_client).await;

    pretty_assertions::assert_eq!(before, snapshot::<MemMetaStorage>(scratch_client).await);

    // And back, the way the dump lands the rebuild's result.
    transfer::<MemMetaStorage, PsqlMetaStorage>(scratch_client, psql_client).await;

    pretty_assertions::assert_eq!(before, snapshot::<PsqlMetaStorage>(psql_client).await);
}
