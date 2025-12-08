use syn::parse_quote;

use crate::configs::DimensionConfig;
use crate::operon_ident;
use crate::utils::{as_lit_str, dimension_metadata_ident};

/// Generates a metadata function for a dimension.
///
/// # Example
/// ```rust,ignore
/// pub const fn dimension_j_meta() -> operon::schema::DimensionMetadata<1usize> {
///     operon::schema::DimensionMetadata {
///         id: "j",
///         deps: ["i"],
///     }
/// }
/// ```
pub fn dimension_metadata(dimension: &DimensionConfig) -> syn::ItemFn {
    let operon = operon_ident();
    let fn_name = dimension_metadata_ident(&dimension.id);
    let n = dimension.depends_on.len();
    let id = as_lit_str(&dimension.id);
    let deps = dimension.depends_on.iter().map(as_lit_str);

    parse_quote! {
        pub const fn #fn_name() -> #operon::schema::DimensionMetadata<#n> {
            #operon::schema::DimensionMetadata {
                id: #id,
                deps: [#(#deps),*],
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::dimension_j;

    #[rstest]
    #[case(dimension_j(), "metadata/dimension.rs")]
    fn test_dimension_metadata(#[case] job: DimensionConfig, #[case] fixture_path: &str) {
        let result = dimension_metadata(&job);
        assert_item_eq(&result, fixture_path);
    }
}
