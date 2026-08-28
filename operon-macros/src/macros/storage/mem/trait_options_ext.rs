use syn::parse_quote;

use crate::utils::{mem_storage_ident, mem_storage_options_ext_ident};

/// Generates the extension trait that builds the in-memory storage from
/// `operon::options::MemStorageOptions`.
///
/// The options type lives in the `operon` crate, so the `build` constructor has to be attached
/// through a trait defined next to the generated storage.
///
/// # Example
/// ```rust,ignore
/// pub trait MemCookingStorageOptionsExt {
///     fn build(self) -> MemCookingStorage;
/// }
/// ```
pub(super) fn trait_options_ext(service_id: &syn::Ident) -> syn::ItemTrait {
    let mem_storage_ident = mem_storage_ident(service_id);
    let options_ext_ident = mem_storage_options_ext_ident(service_id);

    parse_quote! {
        /// Builds the in-memory entity storage for this pipeline from `MemStorageOptions`.
        ///
        /// ```rust,ignore
        /// let storage = MemStorageOptions::new().build();
        /// ```
        pub trait #options_ext_ident {
            /// Builds the in-memory entity storage described by these options.
            fn build(self) -> #mem_storage_ident;
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::service_id;

    #[rstest]
    fn test_trait_options_ext(service_id: syn::Ident) {
        let item = trait_options_ext(&service_id);
        assert_item_eq(&item, "storage/mem/trait_options_ext.rs");
    }
}
