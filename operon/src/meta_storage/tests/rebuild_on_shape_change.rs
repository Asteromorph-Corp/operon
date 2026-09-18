//! Tests for the table rebuilds triggered by a shape change in Postgres.
//!
//! Each sequence initializes a table under one shape, populates it, then initializes it again under
//! another shape and asserts the table that comes back matches the new shape.
//! The assertions read `information_schema` and write through the new shape rather than comparing
//! statement text, since a rebuild that names the wrong relation still generates well-formed SQL.
//!
//! These tests need a Postgres to build the tables in and are skipped when `POSTGRES_URI` is unset.
//! They each own a schema outright, which they drop before the sequence.

use crate::meta_storage::psql::PsqlClient;
use crate::meta_storage::tests::utils::psql_backend;
use crate::meta_storage::{MetaBackend, MetaConnApi, MetaResolutionApi, MetaTicketApi};
use crate::schema::{
    DimensionMetadata, Job, Resolution, TableShape, TaskMetadata, Ticket, TicketStatus,
};

/// The schema the ticket sequence owns outright.
const TICKET_SCHEMA: &str = "operon_rebuild_ticket";

/// The schema the dimension sequence owns outright.
const DIMENSION_SCHEMA: &str = "operon_rebuild_dimension";

/// The schema the unrecorded-shape sequence owns outright.
const UNRECORDED_SCHEMA: &str = "operon_rebuild_unrecorded";

/// The schema the ticket summary sequence owns outright.
const SUMMARY_SCHEMA: &str = "operon_rebuild_summary";

/// The task `delta`, over `i` alone.
fn delta_over_i() -> TaskMetadata<1> {
    TaskMetadata {
        id: "delta",
        dims: ["i"],
        spawn_dim: None,
        priority: &[],
    }
}

/// The task `delta` again, widened to `i` and `j`.
fn delta_over_i_j() -> TaskMetadata<2> {
    TaskMetadata {
        id: "delta",
        dims: ["i", "j"],
        spawn_dim: None,
        priority: &[],
    }
}

/// The dimension `j`, resolved once for the whole run.
fn dim_j() -> DimensionMetadata<0> {
    DimensionMetadata { id: "j", deps: [] }
}

/// The dimension `j` again, widened to resolve per `i`.
fn dim_j_over_i() -> DimensionMetadata<1> {
    DimensionMetadata {
        id: "j",
        deps: ["i"],
    }
}

/// Drops the schema the sequence owns, so it starts from nothing recorded.
async fn drop_schema(client: PsqlClient<'_>, schema: &str) {
    let stmt = format!("DROP SCHEMA IF EXISTS {schema} CASCADE;");
    let _num_rows = client.execute(&stmt, &[]).await.expect("drop the schema");
}

/// The columns of a table, in the order they were declared.
async fn columns(client: PsqlClient<'_>, schema: &'static str, table: &'static str) -> Vec<String> {
    let stmt = "SELECT column_name
                FROM information_schema.columns
                WHERE table_schema = $1 AND table_name = $2
                ORDER BY ordinal_position;";
    client
        .query(stmt, &[&schema, &table])
        .await
        .expect("read the columns")
        .iter()
        .map(|row| row.get(0))
        .collect()
}

#[tokio::test]
async fn ticket_table_is_rebuilt_when_a_task_gains_a_dimension() {
    let Some(psql) = psql_backend(TICKET_SCHEMA) else {
        eprintln!("skipping: POSTGRES_URI is not set, so there is no Postgres to build tables in");
        return;
    };

    let conn = psql.scheduler_conn().await.expect("scheduler conn");
    let client = conn.as_client();

    drop_schema(client, TICKET_SCHEMA).await;
    client.init_schema().await.expect("init_schema");
    client.init_ticket_hash().await.expect("init_ticket_hash");
    client
        .init_ticket_status_type()
        .await
        .expect("init_ticket_status_type");
    client
        .init_ticket_summary()
        .await
        .expect("init_ticket_summary");

    // A run over `i` alone, carried to a ticket the summary counts.
    let _ = client
        .ticket(delta_over_i())
        .init()
        .await
        .expect("narrow init");
    client
        .ticket(delta_over_i())
        .put(Ticket::new(0).with_coordinate::<0>(0))
        .await
        .expect("narrow put");
    client
        .ticket(delta_over_i())
        .mark_done(Job { coordinate: [0] })
        .await
        .expect("narrow mark_done");
    assert_eq!(
        client
            .ticket(delta_over_i())
            .get_status()
            .await
            .expect("narrow status"),
        (1, 0, 0)
    );

    // The same task, widened. The init rebuilds its table and reports the discard.
    assert_eq!(
        client
            .ticket(delta_over_i_j())
            .init()
            .await
            .expect("wide init"),
        TableShape::STALE
    );
    assert_eq!(
        client
            .ticket(delta_over_i_j())
            .init()
            .await
            .expect("wide init again"),
        TableShape::CURRENT
    );

    assert_eq!(
        columns(client, TICKET_SCHEMA, "ticket_delta").await,
        ["i", "j", "deps_done", "deps_quota", "status"]
    );

    // The rebuilt table is empty, and the summary no longer counts the tickets it discarded.
    assert!(
        client
            .ticket(delta_over_i_j())
            .get_all(TicketStatus::Done)
            .await
            .expect("wide get_all")
            .is_empty()
    );
    assert_eq!(
        client
            .ticket(delta_over_i_j())
            .get_status()
            .await
            .expect("wide status"),
        (0, 0, 0)
    );

    // A write through the new shape is what the stale table used to reject.
    client
        .ticket(delta_over_i_j())
        .put(
            Ticket::new(0)
                .with_coordinate::<0>(0)
                .with_coordinate::<1>(1),
        )
        .await
        .expect("wide put");
    assert_eq!(
        client
            .ticket(delta_over_i_j())
            .get_status()
            .await
            .expect("wide status after put"),
        (0, 1, 0)
    );
}

#[tokio::test]
async fn resolution_table_is_rebuilt_when_a_dimension_gains_a_dependency() {
    let Some(psql) = psql_backend(DIMENSION_SCHEMA) else {
        eprintln!("skipping: POSTGRES_URI is not set, so there is no Postgres to build tables in");
        return;
    };

    let conn = psql.scheduler_conn().await.expect("scheduler conn");
    let client = conn.as_client();

    drop_schema(client, DIMENSION_SCHEMA).await;
    client.init_schema().await.expect("init_schema");
    client
        .init_dimension_hash()
        .await
        .expect("init_dimension_hash");

    // A run resolving `j` once, carried to a resolution.
    let _ = client
        .resolution(dim_j())
        .init()
        .await
        .expect("narrow init");
    client
        .resolution(dim_j())
        .put(Resolution {
            coordinate: [],
            ub: 2,
        })
        .await
        .expect("narrow put");

    // The same dimension, widened. The init rebuilds its table and reports the discard.
    assert_eq!(
        client
            .resolution(dim_j_over_i())
            .init()
            .await
            .expect("wide init"),
        TableShape::STALE
    );
    assert_eq!(
        client
            .resolution(dim_j_over_i())
            .init()
            .await
            .expect("wide init again"),
        TableShape::CURRENT
    );

    assert_eq!(
        columns(client, DIMENSION_SCHEMA, "dimension_j").await,
        ["i", "ub"]
    );

    // The rebuilt table is empty, and takes a write through the new shape.
    assert!(
        client
            .resolution(dim_j_over_i())
            .get([0])
            .await
            .expect("wide get")
            .is_none()
    );
    client
        .resolution(dim_j_over_i())
        .put(Resolution {
            coordinate: [0],
            ub: 3,
        })
        .await
        .expect("wide put");
    let resolution = client
        .resolution(dim_j_over_i())
        .get([0])
        .await
        .expect("wide get after put")
        .expect("a resolution at i = 0");
    assert_eq!((resolution.coordinate, resolution.ub), ([0], 3));
}

#[tokio::test]
async fn ticket_summary_is_rebuilt_when_its_shape_changes() {
    let Some(psql) = psql_backend(SUMMARY_SCHEMA) else {
        eprintln!("skipping: POSTGRES_URI is not set, so there is no Postgres to build tables in");
        return;
    };

    let conn = psql.scheduler_conn().await.expect("scheduler conn");
    let client = conn.as_client();

    drop_schema(client, SUMMARY_SCHEMA).await;
    client.init_schema().await.expect("init_schema");
    client.init_ticket_hash().await.expect("init_ticket_hash");
    client
        .init_ticket_status_type()
        .await
        .expect("init_ticket_status_type");
    client
        .init_ticket_summary()
        .await
        .expect("first init_ticket_summary");

    // A run over `i`, carried to one ticket the summary counts done and one it counts queued.
    let _ = client
        .ticket(delta_over_i())
        .init()
        .await
        .expect("first init");
    for coordinate in [0, 1] {
        client
            .ticket(delta_over_i())
            .put(Ticket::new(0).with_coordinate::<0>(coordinate))
            .await
            .expect("put");
    }
    client
        .ticket(delta_over_i())
        .mark_done(Job { coordinate: [0] })
        .await
        .expect("mark_done");
    assert_eq!(
        client
            .ticket(delta_over_i())
            .get_status()
            .await
            .expect("status before the rebuild"),
        (1, 1, 0)
    );

    // The summary as a release naming its key column after a job left it, vouched for by nothing.
    let stmt = format!(
        "ALTER TABLE {SUMMARY_SCHEMA}.ticket_summary RENAME COLUMN task_id TO job_id;
         DELETE FROM {SUMMARY_SCHEMA}._ticket_summary_hash;"
    );
    client
        .batch_execute(&stmt)
        .await
        .expect("spell the summary as the earlier release did");

    // The init cannot trust the table it finds, so it rebuilds it under the shape it knows.
    client
        .init_ticket_summary()
        .await
        .expect("second init_ticket_summary");
    assert_eq!(
        columns(client, SUMMARY_SCHEMA, "ticket_summary").await,
        ["task_id", "waiting", "queued", "done"]
    );

    // The tickets outlived the counters, so the init that keeps their table counts them again.
    assert_eq!(
        client
            .ticket(delta_over_i())
            .init()
            .await
            .expect("second init"),
        TableShape::CURRENT
    );
    assert_eq!(
        client
            .ticket(delta_over_i())
            .get_status()
            .await
            .expect("status after the rebuild"),
        (1, 1, 0)
    );

    // The counters the rebuilt table holds move with the tickets, as the reinstated triggers keep
    // them.
    client
        .ticket(delta_over_i())
        .mark_done(Job { coordinate: [1] })
        .await
        .expect("mark_done after the rebuild");
    assert_eq!(
        client
            .ticket(delta_over_i())
            .get_status()
            .await
            .expect("status after the second mark_done"),
        (2, 0, 0)
    );
}

#[tokio::test]
async fn ticket_table_is_rebuilt_when_its_shape_is_unrecorded() {
    let Some(psql) = psql_backend(UNRECORDED_SCHEMA) else {
        eprintln!("skipping: POSTGRES_URI is not set, so there is no Postgres to build tables in");
        return;
    };

    let conn = psql.scheduler_conn().await.expect("scheduler conn");
    let client = conn.as_client();

    drop_schema(client, UNRECORDED_SCHEMA).await;
    client.init_schema().await.expect("init_schema");
    client.init_ticket_hash().await.expect("init_ticket_hash");
    client
        .init_ticket_status_type()
        .await
        .expect("init_ticket_status_type");
    client
        .init_ticket_summary()
        .await
        .expect("init_ticket_summary");

    let _ = client
        .ticket(delta_over_i())
        .init()
        .await
        .expect("recorded init");
    client
        .ticket(delta_over_i())
        .put(Ticket::new(0).with_coordinate::<0>(0))
        .await
        .expect("recorded put");

    // A release predating the shape record leaves its table standing with nothing vouching for it.
    let stmt = format!("DELETE FROM {UNRECORDED_SCHEMA}._ticket_hash WHERE id = 'delta';");
    let _ = client
        .execute(&stmt, &[])
        .await
        .expect("drop the shape record");

    // The init cannot trust the table it finds, so it rebuilds it under the shape it knows.
    assert_eq!(
        client
            .ticket(delta_over_i())
            .init()
            .await
            .expect("unrecorded init"),
        TableShape::STALE
    );
    assert!(
        client
            .ticket(delta_over_i())
            .get_all(TicketStatus::Waiting)
            .await
            .expect("unrecorded get_all")
            .is_empty()
    );

    // The shape is recorded again, so a further init keeps what the rebuild left.
    assert_eq!(
        client
            .ticket(delta_over_i())
            .init()
            .await
            .expect("unrecorded init again"),
        TableShape::CURRENT
    );
}
