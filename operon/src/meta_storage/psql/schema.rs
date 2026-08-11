use crate::meta_storage::psql::PsqlClient;
use crate::meta_storage::psql::error::PsqlResult;
use crate::meta_storage::psql::resolution::DIMENSION_SHAPES;
use crate::meta_storage::psql::ticket::TICKET_SHAPES;
use crate::utils::{
    SchemaPrefix, ShapeAction, ShapeRecord, ShapeTable, build_tables, hash_metadata,
    init_shape_table_query, shape_query,
};

/// The table holding the per-task ticket counters.
const SUMMARY_TABLE: &str = "ticket_summary";

/// The table recording the ticket summary's shape ID.
const SUMMARY_SHAPES: ShapeTable<'static> = ShapeTable {
    table: "_ticket_summary_hash",
    column: "hash",
};

/// The pointer to the ticket summary's shape ID.
const SUMMARY_RECORD: ShapeRecord<'static> = SUMMARY_SHAPES.record(SUMMARY_TABLE);

/// The DDL for the ticket summary table and the trigger function.
fn init_summary_query(schema_prefix: SchemaPrefix<'_>) -> String {
    let table_stmt = format!(
        "CREATE TABLE IF NOT EXISTS {schema_prefix}{SUMMARY_TABLE} (
            task_id TEXT NOT NULL,
            waiting BIGINT NOT NULL DEFAULT 0,
            queued BIGINT NOT NULL DEFAULT 0,
            done BIGINT NOT NULL DEFAULT 0,
            PRIMARY KEY (task_id),
            CHECK (
                waiting >= 0 AND
                queued >= 0 AND
                done >= 0
            )
        );"
    );
    let func_stmt = format!(
        "CREATE OR REPLACE FUNCTION {schema_prefix}trg_ticket_summary() RETURNS TRIGGER AS $$
        BEGIN
            IF TG_OP = 'INSERT' THEN
                UPDATE {schema_prefix}{SUMMARY_TABLE} s
                SET
                    waiting = waiting + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'waiting'),
                    queued  = queued  + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'queued'),
                    done    = done    + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'done')
                WHERE task_id = TG_ARGV[0];

            ELSIF TG_OP = 'UPDATE' THEN
                UPDATE {schema_prefix}{SUMMARY_TABLE}
                SET
                    waiting = waiting
                                - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'waiting')
                                + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'waiting'),
                    queued  = queued
                                - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'queued')
                                + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'queued'),
                    done    = done
                                - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'done')
                                + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'done')
                WHERE task_id = TG_ARGV[0];

            ELSIF TG_OP = 'DELETE' THEN
                UPDATE {schema_prefix}{SUMMARY_TABLE}
                SET
                    waiting = waiting - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'waiting'),
                    queued  = queued  - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'queued'),
                    done    = done    - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'done')
                WHERE task_id = TG_ARGV[0];

            ELSIF TG_OP = 'TRUNCATE' THEN
                UPDATE {schema_prefix}{SUMMARY_TABLE}
                SET waiting = 0, queued = 0, done = 0
                WHERE task_id = TG_ARGV[0];
            END IF;

            RETURN NULL;
        END;
        $$ LANGUAGE plpgsql;"
    );

    format!("{table_stmt}\n\n{func_stmt}")
}

impl PsqlClient<'_> {
    /// Initializes the metadata storage schema, if specified.
    pub async fn init_schema(&self) -> PsqlResult<()> {
        let Some(schema) = self.schema() else {
            return Ok(());
        };
        let create_schema = format!("CREATE SCHEMA IF NOT EXISTS {schema}");
        self.execute(&create_schema, &[]).await?;
        Ok(())
    }

    /// Initializes the table recording each task's ticket shape.
    pub async fn init_ticket_hash(&self) -> PsqlResult<()> {
        let stmt = init_shape_table_query(TICKET_SHAPES, self.schema_prefix());
        self.execute(&stmt, &[]).await?;
        Ok(())
    }

    /// Initializes the table recording each dimension's shape.
    pub async fn init_dimension_hash(&self) -> PsqlResult<()> {
        let stmt = init_shape_table_query(DIMENSION_SHAPES, self.schema_prefix());
        self.execute(&stmt, &[]).await?;
        Ok(())
    }

    /// Initializes the `ticket_status` type.
    pub async fn init_ticket_status_type(&self) -> PsqlResult<()> {
        let schema = self.schema_prefix();
        let create_status_type = format!(
            "DO $$ BEGIN
                CREATE TYPE {schema}ticket_status AS ENUM (
                    'waiting',
                    'queued',
                    'done'
                );
            EXCEPTION
                WHEN duplicate_object THEN null;
            END $$;"
        );
        self.execute(&create_status_type, &[]).await?;
        Ok(())
    }

    /// Initializes the ticket summary table.
    ///
    /// Rebuilds the table when the shape has changed.
    /// Each task's [`init`](crate::meta_storage::MetaTicketApi::init) call will initialize the
    /// summary row even if the table was rebuilt.
    pub async fn init_ticket_summary(&self) -> PsqlResult<()> {
        let schema_prefix = self.schema_prefix();

        let init_record = init_shape_table_query(SUMMARY_SHAPES, schema_prefix);
        self.execute(&init_record, &[]).await?;

        let init_query = init_summary_query(schema_prefix);
        let shape_id = hash_metadata(&init_query);
        let shape_stmt = shape_query(SUMMARY_RECORD, &[SUMMARY_TABLE], schema_prefix);
        let row = self.query_opt(&shape_stmt, &[]).await?;
        let action = ShapeAction::from_row(row.as_ref(), &shape_id);

        if let Some(stmt) = build_tables(
            SUMMARY_RECORD,
            &[SUMMARY_TABLE],
            &shape_id,
            schema_prefix,
            action,
            init_query,
        ) {
            self.batch_execute(&stmt).await?;
        }
        Ok(())
    }
}
