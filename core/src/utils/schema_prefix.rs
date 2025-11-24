#[derive(Debug, Clone, Copy)]
pub struct SchemaPrefix<'a>(pub Option<&'a str>);

#[derive(Debug, Clone)]
pub struct SchemaPrefixOwned(pub Option<String>);

impl SchemaPrefix<'_> {
    pub fn into_owned(&self) -> SchemaPrefixOwned {
        SchemaPrefixOwned(self.0.map(|s| s.to_owned()))
    }
}

impl std::fmt::Display for SchemaPrefix<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(schema) = self.0 {
            write!(f, "{schema}.")
        } else {
            Ok(())
        }
    }
}

impl std::fmt::Display for SchemaPrefixOwned {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(schema) = &self.0 {
            write!(f, "{schema}.")
        } else {
            Ok(())
        }
    }
}
