use crate::configs::DimensionConfig;

pub struct ClearResolutionQuery<'a>(pub(super) &'a DimensionConfig);

impl std::fmt::Display for ClearResolutionQuery<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TRUNCATE TABLE {{schema_prefix}}dimension_{};",
            self.0.id
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clear_resolution_query() {
        let dimension_i = DimensionConfig {
            id: "i".to_string(),
            depends_on: vec![],
        };

        let clear_resolution = ClearResolutionQuery(&dimension_i);

        let stmt_i = "TRUNCATE TABLE {schema_prefix}dimension_i;";

        assert_eq!(clear_resolution.to_string(), stmt_i);
    }
}
