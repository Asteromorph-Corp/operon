async fn put_all_a(
    &self,
    entity: operon::schema::Entity<0usize, Vec<A>>,
) -> Result<(), operon::storage::StorageError> {
    let [] = entity.coordinate;
    let mut conn = self.pool.get().await?;
    let tx = conn.transaction().await?;
    let schema_prefix = operon::utils::SchemaPrefix(self.schema.as_deref());

    let temp_table_stmt =
        format!("CREATE TEMP TABLE temp (LIKE {schema_prefix}a INCLUDING ALL) ON COMMIT DROP;");
    tx.execute(&temp_table_stmt, &[]).await?;

    let mut writer = operon::csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(vec![]);
    for (i, value) in entity.value.iter().enumerate() {
        writer.serialize((i, operon::serde_json::to_value(value)?.to_string()))?;
    }
    let copy_stmt = "COPY temp (i, value) FROM STDIN WITH (FORMAT csv);";
    let sink = tx.copy_in(copy_stmt).await?;
    let mut sink = Box::pin(sink);
    operon::futures::sink::SinkExt::send(
        &mut sink,
        operon::bytes::Bytes::from(writer.into_inner()?),
    )
    .await?;
    operon::futures::sink::SinkExt::close(&mut sink).await?;

    let insert_stmt = format!(
        "INSERT INTO {schema_prefix}a (i, value)\nSELECT i, value FROM temp\nON CONFLICT (i) DO UPDATE SET value = EXCLUDED.value;"
    );
    tx.execute(&insert_stmt, &[]).await?;
    tx.commit().await?;
    Ok(())
}

async fn put_all_b(
    &self,
    entity: operon::schema::Entity<1usize, Vec<B>>,
) -> Result<(), operon::storage::StorageError> {
    let [i] = entity.coordinate;

    let mut conn = self.pool.get().await?;
    let tx = conn.transaction().await?;
    let schema_prefix = operon::utils::SchemaPrefix(self.schema.as_deref());

    let temp_table_stmt =
        format!("CREATE TEMP TABLE temp (LIKE {schema_prefix}b INCLUDING ALL) ON COMMIT DROP;");
    tx.execute(&temp_table_stmt, &[]).await?;

    let mut writer = operon::csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(vec![]);
    for (j, value) in entity.value.iter().enumerate() {
        writer.serialize((i, j, operon::serde_json::to_value(value)?.to_string()))?;
    }
    let copy_stmt = "COPY temp (i, j, value) FROM STDIN WITH (FORMAT csv);";
    let sink = tx.copy_in(copy_stmt).await?;
    let mut sink = Box::pin(sink);
    operon::futures::sink::SinkExt::send(
        &mut sink,
        operon::bytes::Bytes::from(writer.into_inner()?),
    )
    .await?;
    operon::futures::sink::SinkExt::close(&mut sink).await?;

    let insert_stmt = format!(
        "INSERT INTO {schema_prefix}b (i, j, value)\nSELECT i, j, value FROM temp\nON CONFLICT (i, j) DO UPDATE SET value = EXCLUDED.value;"
    );
    tx.execute(&insert_stmt, &[]).await?;
    tx.commit().await?;
    Ok(())
}

async fn put_all_c(
    &self,
    entity: operon::schema::Entity<1usize, Vec<C>>,
) -> Result<(), operon::storage::StorageError> {
    let [i] = entity.coordinate;

    let mut conn = self.pool.get().await?;
    let tx = conn.transaction().await?;
    let schema_prefix = operon::utils::SchemaPrefix(self.schema.as_deref());
    let temp_table_stmt =
        format!("CREATE TEMP TABLE temp (LIKE {schema_prefix}c INCLUDING ALL) ON COMMIT DROP;");
    tx.execute(&temp_table_stmt, &[]).await?;
    let mut writer = operon::csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(vec![]);
    for (k, value) in entity.value.iter().enumerate() {
        writer.serialize((i, k, operon::serde_json::to_value(value)?.to_string()))?;
    }
    let copy_stmt = "COPY temp (i, k, value) FROM STDIN WITH (FORMAT csv);";
    let sink = tx.copy_in(copy_stmt).await?;
    let mut sink = Box::pin(sink);
    operon::futures::sink::SinkExt::send(
        &mut sink,
        operon::bytes::Bytes::from(writer.into_inner()?),
    )
    .await?;
    operon::futures::sink::SinkExt::close(&mut sink).await?;
    let insert_stmt = format!(
        "INSERT INTO {schema_prefix}c (i, k, value)\nSELECT i, k, value FROM temp\nON CONFLICT (i, k) DO UPDATE SET value = EXCLUDED.value;"
    );
    tx.execute(&insert_stmt, &[]).await?;
    tx.commit().await?;
    Ok(())
}
