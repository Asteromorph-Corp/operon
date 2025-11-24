impl operon::storage::psql::EntityQueries for CookingEntities {
    fn init_stmt(&self, schema: operon::utils::SchemaPrefix<'_>) -> String {
        [
            self.a.init_stmt(schema),
            self.b.init_stmt(schema),
            self.c.init_stmt(schema),
            self.d.init_stmt(schema),
            self.e.init_stmt(schema),
            self.f.init_stmt(schema),
        ]
        .join("\n")
    }
    fn clear_stmt(&self, schema: operon::utils::SchemaPrefix<'_>) -> String {
        let tables = [
            self.a.id, self.b.id, self.c.id, self.d.id, self.e.id, self.f.id,
        ]
        .map(|t| format!("{schema}{t}"))
        .join(",");
        format!("TRUNCATE TABLE {tables};")
    }
}
