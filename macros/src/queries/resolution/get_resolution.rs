use crate::configs::DimensionConfig;

pub struct GetResolutionQuery<'a>(pub(super) &'a DimensionConfig);

impl std::fmt::Display for GetResolutionQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SELECT {}_ub FROM {{schema_prefix}}dimension_{}",
            self.0.id, self.0.id
        )?;
        if let Some(dep) = self.0.depends_on.first() {
            write!(f, " WHERE {dep} = $1")?;
        }
        for (i, dep) in self.0.depends_on.iter().enumerate().skip(1) {
            write!(f, " AND {} = ${}", dep, i + 1)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_resolution_query_no_dependency() {
        let dimension_i = DimensionConfig {
            id: "i".to_string(),
            depends_on: vec![],
        };

        let get_resolution = dimension_i.get_resolution_query();

        let stmt_i = "SELECT i_ub FROM {schema_prefix}dimension_i";

        assert_eq!(get_resolution.to_string(), stmt_i);
    }

    #[test]
    fn test_get_resolution_query_with_dependency() {
        let dimension_l = DimensionConfig {
            id: "l".to_string(),
            depends_on: vec!["j".to_string(), "k".to_string()],
        };

        let get_resolution = dimension_l.get_resolution_query();

        let stmt_l = "SELECT l_ub FROM {schema_prefix}dimension_l WHERE j = $1 AND k = $2";

        assert_eq!(get_resolution.to_string(), stmt_l);
    }
}
