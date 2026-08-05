//! Tests for the footprint table rebuild triggered by a version change in Postgres.
//!
//! Each sequence records a run under some footprint version, initializes the tables again, and
//! asserts what survives.
//! Tables predating versioning and tables recorded under another version are rebuilt, while
//! tables at the current version keep their run.
//! The assertions write and read through the tables rather than comparing statement text, since a
//! stale constraint only shows up on a write.
//!
//! These tests need a Postgres to build the tables in and are skipped when `POSTGRES_URI` is unset.
//! They each own a schema outright, which they drop before the sequence.

use indoc::formatdoc;
use uuid::Uuid;

use crate::meta_storage::psql::PsqlClient;
use crate::meta_storage::tests::utils::psql_backend;
use crate::meta_storage::{MetaBackend, MetaConnApi};
use crate::schema::{RunFootprint, RunState};

/// The schema the rebuild sequence owns outright.
const REBUILD_SCHEMA: &str = "operon_rebuild_footprint";

/// The schema the same-version sequence owns outright.
const SAME_VERSION_SCHEMA: &str = "operon_rebuild_footprint_same";

/// The schema the version-bump sequence owns outright.
const BUMPED_VERSION_SCHEMA: &str = "operon_rebuild_footprint_bumped";

/// Drops the schema the sequence owns, so it starts from nothing recorded.
async fn drop_schema(client: PsqlClient<'_>, schema: &str) {
    let stmt = format!("DROP SCHEMA IF EXISTS {schema} CASCADE;");
    client.execute(&stmt, &[]).await.expect("drop the schema");
}

/// Builds the footprint tables as a release predating footprint versioning would, recording a
/// gracefully stopped run as `"paused"`.
async fn init_unversioned_footprint(client: PsqlClient<'_>, schema: &str, run_id: Uuid) {
    let stmt = formatdoc! {"
        CREATE TABLE {schema}.runs (
            key TEXT PRIMARY KEY CHECK (key = 'global'),
            run_id UUID NOT NULL UNIQUE,
            created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
            finished_at TIMESTAMP WITH TIME ZONE DEFAULT NULL,
            updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
            state TEXT NOT NULL DEFAULT 'running' CHECK (
                state IN ('running', 'paused', 'completed', 'aborted')
            )
        );

        CREATE TABLE {schema}.run_executions (
            execution_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            run_id UUID REFERENCES {schema}.runs(run_id) ON DELETE CASCADE,
            started_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
            ended_at TIMESTAMP WITH TIME ZONE,
            end_reason TEXT CHECK (
                end_reason IN (NULL, 'paused', 'completed', 'aborted')
            )
        );

        INSERT INTO {schema}.runs (key, run_id, state)
        VALUES ('global', '{run_id}', 'paused');

        INSERT INTO {schema}.run_executions (run_id, end_reason)
        VALUES ('{run_id}', 'paused');"
    };
    client
        .batch_execute(&stmt)
        .await
        .expect("build the unversioned footprint tables");
}

/// Overwrites the footprint version recorded for the tables.
async fn set_recorded_version(client: PsqlClient<'_>, schema: &str, version: &str) {
    let stmt =
        format!("UPDATE {schema}._footprint_version SET version = '{version}' WHERE id = 'runs';");
    client
        .execute(&stmt, &[])
        .await
        .expect("record another version");
}

/// How many rows a footprint table holds.
async fn row_count(client: PsqlClient<'_>, schema: &str, table: &str) -> i64 {
    let stmt = format!("SELECT COUNT(*) FROM {schema}.{table};");
    let rows = client.query(&stmt, &[]).await.expect("count the rows");
    rows.first().expect("a count comes back").get(0)
}

#[tokio::test]
async fn footprint_tables_are_rebuilt_when_they_predate_versioning() {
    let Some(psql) = psql_backend(REBUILD_SCHEMA) else {
        eprintln!("skipping: POSTGRES_URI is not set, so there is no Postgres to build tables in");
        return;
    };

    let conn = psql.scheduler_conn().await.expect("scheduler conn");
    let client = conn.as_client();

    drop_schema(client, REBUILD_SCHEMA).await;
    client.init_schema().await.expect("init_schema");

    // A gracefully stopped run, spelled as a release predating versioning would.
    let run_id = Uuid::new_v4();
    init_unversioned_footprint(client, REBUILD_SCHEMA, run_id).await;
    assert_eq!(row_count(client, REBUILD_SCHEMA, "runs").await, 1);
    assert_eq!(row_count(client, REBUILD_SCHEMA, "run_executions").await, 1);

    // The init rebuilds both tables, discarding the run and its execution history.
    client.init_footprint().await.expect("init_footprint");
    assert_eq!(row_count(client, REBUILD_SCHEMA, "runs").await, 0);
    assert_eq!(row_count(client, REBUILD_SCHEMA, "run_executions").await, 0);
    assert!(
        client
            .get_footprint()
            .await
            .expect("get_footprint after rebuild")
            .is_none()
    );

    // A graceful stop is what the unversioned tables rejected.
    let footprint = RunFootprint::new(Uuid::new_v4(), RunState::Stopped);
    client
        .upsert_run(&footprint)
        .await
        .expect("record a graceful stop");
    assert_eq!(
        client
            .get_footprint()
            .await
            .expect("get_footprint after the stop")
            .expect("a footprint was recorded")
            .metadata
            .state,
        RunState::Stopped
    );
}

#[tokio::test]
async fn footprint_tables_are_rebuilt_when_the_recorded_version_differs() {
    let Some(psql) = psql_backend(BUMPED_VERSION_SCHEMA) else {
        eprintln!("skipping: POSTGRES_URI is not set, so there is no Postgres to build tables in");
        return;
    };

    let conn = psql.scheduler_conn().await.expect("scheduler conn");
    let client = conn.as_client();

    drop_schema(client, BUMPED_VERSION_SCHEMA).await;
    client.init_schema().await.expect("init_schema");
    client.init_footprint().await.expect("first init_footprint");

    let footprint = RunFootprint::new(Uuid::new_v4(), RunState::Stopped);
    client.upsert_run(&footprint).await.expect("record a run");
    client
        .put_execution(footprint.metadata.run_id, Uuid::new_v4())
        .await
        .expect("record an execution");

    // A release that bumped the version leaves the tables recorded under the old one.
    set_recorded_version(client, BUMPED_VERSION_SCHEMA, "0").await;
    client
        .init_footprint()
        .await
        .expect("second init_footprint");

    assert!(
        client
            .get_footprint()
            .await
            .expect("get_footprint after rebuild")
            .is_none()
    );
    assert_eq!(
        row_count(client, BUMPED_VERSION_SCHEMA, "run_executions").await,
        0
    );
}

#[tokio::test]
async fn footprint_tables_survive_an_init_at_the_same_version() {
    let Some(psql) = psql_backend(SAME_VERSION_SCHEMA) else {
        eprintln!("skipping: POSTGRES_URI is not set, so there is no Postgres to build tables in");
        return;
    };

    let conn = psql.scheduler_conn().await.expect("scheduler conn");
    let client = conn.as_client();

    drop_schema(client, SAME_VERSION_SCHEMA).await;
    client.init_schema().await.expect("init_schema");
    client.init_footprint().await.expect("first init_footprint");

    let footprint = RunFootprint::new(Uuid::new_v4(), RunState::Stopped);
    client.upsert_run(&footprint).await.expect("record a run");
    client
        .put_execution(footprint.metadata.run_id, Uuid::new_v4())
        .await
        .expect("record an execution");

    // The version is unchanged, so the run survives the second init.
    client
        .init_footprint()
        .await
        .expect("second init_footprint");
    assert_eq!(
        client
            .get_footprint()
            .await
            .expect("get_footprint after the second init")
            .expect("the run is still recorded")
            .metadata,
        footprint.metadata
    );
    assert_eq!(
        row_count(client, SAME_VERSION_SCHEMA, "run_executions").await,
        1
    );
}
