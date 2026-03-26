use syn::parse_quote;

use crate::configs::EntityConfig;
use crate::operon_ident;
use crate::utils::{entity_metadata_ident, to_lit_str, to_type};

/// Generates a metadata function for an entity.
///
/// # Example
/// ```rust,ignore
/// pub const fn entity_a_meta() -> operon::__private::EntityMetadata<1usize, A> {
///     operon::__private::EntityMetadata {
///         id: "a",
///         dims: ["i"],
///         _phantom: std::marker::PhantomData,
///     }
/// }
/// ```
pub fn entity_metadata(entity: &EntityConfig) -> syn::ItemFn {
    let operon = operon_ident();
    let fn_name = entity_metadata_ident(&entity.id);
    let n = entity.dims.len();
    let ty = to_type(&entity.id);
    let id = to_lit_str(&entity.id);
    let dims = entity.dims.iter().map(to_lit_str);

    parse_quote! {
        pub const fn #fn_name() -> #operon::__private::EntityMetadata<#n, #ty> {
            #operon::__private::EntityMetadata {
                id: #id,
                dims: [#(#dims),*],
                _phantom: std::marker::PhantomData,
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::entity_a;

    #[rstest]
    #[case(entity_a(), "metadata/entity.rs")]
    fn test_entity_metadata(#[case] entity: EntityConfig, #[case] fixture_path: &str) {
        let result = entity_metadata(&entity);
        assert_item_eq(&result, fixture_path);
    }
}
