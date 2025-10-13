use syn::parse_quote;

use crate::configs::DimensionConfig;
use crate::utils::{dimension_ident, resolution_ident};

/// Generates a struct definition for the resolution of a dimension.
///
/// Example:
/// ```rust,ignore
/// #[doc = "A struct representing the resolution of dimension i"]
/// #[derive(Debug, Clone, Copy)]
/// pub struct IResolution(pub IDim, pub JDim, pub KDim);
/// ```
pub(super) fn resolution_definition(dimension: &DimensionConfig) -> syn::ItemStruct {
    let res_ident = resolution_ident(&dimension.id);
    let dim_ident = dimension_ident(&dimension.id);
    let dep_dim_idents = dimension.depends_on.iter().map(dimension_ident);

    let doc = format!(
        "A struct representing the resolution of dimension {}",
        dimension.id
    );

    parse_quote! {
        // TODO: Change this to non-tuple struct.
        #[doc = #doc]
        #[derive(Debug, Clone, Copy)]
        pub struct #res_ident(pub #dim_ident, #(pub #dep_dim_idents),*);
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::{dimension_i, dimension_j};

    #[rstest]
    #[case::simple(dimension_i(), "schema/resolution/resolution_definition.simple.rs")]
    #[case::with_dependency(
        dimension_j(),
        "schema/resolution/resolution_definition.with_dependency.rs"
    )]
    fn test_resolution_definition(#[case] dim: DimensionConfig, #[case] fixture_path: &str) {
        let result = resolution_definition(&dim);
        assert_item_eq(&result, fixture_path);
    }
}
