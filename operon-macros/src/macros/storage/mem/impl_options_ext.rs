use syn::parse_quote;

use crate::operon_ident;
use crate::utils::{mem_storage_ident, mem_storage_options_ext_ident};

/// Generates the implementation of the options extension trait for
/// `operon::options::MemStorageOptions`.
///
/// # Example
/// ```rust,ignore
/// impl MemCookingStorageOptionsExt for operon::options::MemStorageOptions {
///     fn build(self) -> MemCookingStorage {
///         MemCookingStorage::from(self)
///     }
/// }
/// ```
pub(super) fn impl_options_ext(service_id: &syn::Ident) -> syn::ItemImpl {
    let operon = operon_ident();
    let mem_storage_ident = mem_storage_ident(service_id);
    let options_ext_ident = mem_storage_options_ext_ident(service_id);

    parse_quote! {
        impl #options_ext_ident for #operon::options::MemStorageOptions {
            fn build(self) -> #mem_storage_ident {
                #mem_storage_ident::new(self)
            }
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
    fn test_impl_options_ext(service_id: syn::Ident) {
        let item = impl_options_ext(&service_id);
        assert_item_eq(&item, "storage/mem/impl_options_ext.rs");
    }
}
