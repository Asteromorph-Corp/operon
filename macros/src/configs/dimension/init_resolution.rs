use crate::configs::DimensionConfig;

impl DimensionConfig {
    pub fn init_resolution_query(&self) -> InitResolutionQuery<'_> {
        InitResolutionQuery(self)
    }
}

pub struct InitResolutionQuery<'a>(&'a DimensionConfig);

impl std::fmt::Display for InitResolutionQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "CREATE TABLE IF NOT EXISTS {{schema_prefix}}dimension_{} (",
            self.0.id
        )?;
        for dep in &self.0.depends_on {
            writeln!(f, "    {} BIGINT,", dep)?;
        }
        write!(f, "    {}_ub BIGINT NOT NULL", self.0.id)?;
        if !self.0.depends_on.is_empty() {
            writeln!(f, ",")?;
            writeln!(f, "    PRIMARY KEY ({})", self.0.depends_on.join(","))?;
        } else {
            writeln!(f, "")?;
        }
        write!(f, ");")
    }
}

#[cfg(test)]
mod tests {
    use indoc::{formatdoc, indoc};

    use super::*;

    #[test]
    fn test_init_resolution_query_no_dependency() {
        let dimension_i = DimensionConfig {
            id: "i".to_string(),
            depends_on: vec![],
        };

        let init_resolution = dimension_i.init_resolution_query();

        let stmt_i = indoc! {"
            CREATE TABLE IF NOT EXISTS {schema_prefix}dimension_i (
                i_ub BIGINT NOT NULL
            );"
        }; // Note: placing this string literal inside the quote! macro results in a `\n` instead of `\\n`, causing the test to fail.

        assert_eq!(init_resolution.to_string(), stmt_i);
    }

    #[test]
    fn test_init_resolution_query_with_dependency() {
        let dimension_i = DimensionConfig {
            id: "j".to_string(),
            depends_on: vec!["i".to_string()],
        };

        let init_resolution = dimension_i.init_resolution_query();

        let stmt_i = indoc! {"
            CREATE TABLE IF NOT EXISTS {schema_prefix}dimension_j (
                i BIGINT,
                j_ub BIGINT NOT NULL,
                PRIMARY KEY (i)
            );"
        };

        assert_eq!(init_resolution.to_string(), stmt_i);
    }
}
