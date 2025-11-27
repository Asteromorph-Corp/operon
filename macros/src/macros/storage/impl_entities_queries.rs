use syn::parse_quote;

use crate::configs::EntityConfigMap;
use crate::operon_ident;
use crate::utils::{entities_ident, variable_ident};

/// Generates the implementation of `EntityQueries` trait for the entity struct.
///
/// # Example
/// ```rust,ignore
/// impl operon::storage::psql::EntityQueries for CookingEntities {
///     fn init_stmt(&self, schema: operon::utils::SchemaPrefix<'_>) -> String {
///         [
///             self.a.init_stmt(schema),
///             self.b.init_stmt(schema),
///             self.c.init_stmt(schema),
///             self.d.init_stmt(schema),
///             self.e.init_stmt(schema),
///             self.f.init_stmt(schema),
///         ]
///         .join("\n")
///     }
///
///     fn clear_stmt(&self, schema: operon::utils::SchemaPrefix<'_>) -> String {
///         let tables = [
///             self.a.id, self.b.id, self.c.id, self.d.id, self.e.id, self.f.id,
///         ]
///         .map(|t| format!("{schema}{t}"))
///         .join(",");
///         format!("TRUNCATE TABLE {tables};")
///     }
/// }
/// ```
pub fn impl_entities_queries(service_id: &str, entities: &EntityConfigMap) -> syn::ItemImpl {
    let operon = operon_ident();
    let entities_ident = entities_ident(service_id);
    let fields = entities
        .keys()
        .map(|entity_id| variable_ident(entity_id))
        .collect::<Vec<_>>();

    parse_quote! {
        impl #operon::storage::psql::EntityQueries for #entities_ident {
            fn init_stmt(&self, schema: #operon::utils::SchemaPrefix<'_>) -> String {
                [#(self.#fields.init_stmt(schema),)*]
                    .join("\n")
            }

            fn clear_stmt(&self, schema: #operon::utils::SchemaPrefix<'_>) -> String {
                let tables = [#(self.#fields.id,)*].map(|t| format!("{schema}{t}")).join(",");
                format!("TRUNCATE TABLE {tables};")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::{all_entities, service_id};

    #[rstest]
    fn test_impl_entities_default(service_id: &str, all_entities: EntityConfigMap) {
        let item = impl_entities_queries(service_id, &all_entities);
        assert_item_eq(&item, "storage/impl_entities_queries.rs");
    }
}
