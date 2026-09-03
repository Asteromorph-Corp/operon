use syn::parse_quote;

use crate::utils::{entities_ident, operon_ident, sql_storage_ident};

/// Generates a type alias for the psql storage.
///
/// # Example
/// ```rust,ignore
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/storage/psql/storage_definition.rs") )]
/// ```
pub(super) fn storage_definition(service_id: &syn::Ident) -> syn::ItemType {
    let operon = operon_ident();
    let sql_storage_ident = sql_storage_ident(service_id);
    let entities_ident = entities_ident(service_id);

    parse_quote! {
        pub type #sql_storage_ident = #operon::__private::PsqlStorage<#entities_ident>;
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::service_id;

    #[rstest]
    fn test_storage_definition(service_id: syn::Ident) {
        let item = storage_definition(&service_id);
        assert_item_eq(&item, "storage/psql/storage_definition.rs");
    }
}
