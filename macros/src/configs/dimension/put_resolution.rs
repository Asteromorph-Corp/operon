use crate::configs::DimensionConfig;

impl DimensionConfig {
    pub fn put_resolution_query(&self) -> PutResolutionQuery<'_> {
        PutResolutionQuery(self)
    }
}

pub struct PutResolutionQuery<'a>(&'a DimensionConfig);

impl std::fmt::Display for PutResolutionQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "INSERT INTO {{schema_prefix}}dimension_{} (", self.0.id)?;
        for dep in &self.0.depends_on {
            write!(f, "{}, ", dep)?;
        }
        write!(f, "{}_ub) VALUES ($1", self.0.id)?;
        for i in 1..(self.0.depends_on.len() + 1) {
            write!(f, ", ${}", i + 1)?;
        }
        write!(f, ") ON CONFLICT DO NOTHING;")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_put_resolution_query_no_dependency() {
        let dimension_i = DimensionConfig {
            id: "i".to_string(),
            depends_on: vec![],
        };

        let put_resolution = dimension_i.put_resolution_query();

        let stmt_i =
            "INSERT INTO {schema_prefix}dimension_i (i_ub) VALUES ($1) ON CONFLICT DO NOTHING;";

        assert_eq!(put_resolution.to_string(), stmt_i);
    }

    #[test]
    fn test_put_resolution_query_with_dependency() {
        let dimension_j = DimensionConfig {
            id: "j".to_string(),
            depends_on: vec!["i".to_string()],
        };

        let put_resolution = dimension_j.put_resolution_query();

        let stmt_j = "INSERT INTO {schema_prefix}dimension_j (i, j_ub) VALUES ($1, $2) ON CONFLICT DO NOTHING;";

        assert_eq!(put_resolution.to_string(), stmt_j);
    }
}
