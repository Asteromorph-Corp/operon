use syn::parse_quote;

use crate::operon_ident;
use crate::utils::mem_storage_ident;

/// Generates the `new` function for the `MemCookingStorage` struct.
///
/// # Example
/// ```rust,ignore
/// impl MemCookingStorage {
///     fn new(_options: operon::options::MemStorageOptions) -> Self {
///         Self::default()
///     }
/// }
/// ```
pub(super) fn impl_new(service_id: &syn::Ident) -> syn::ItemImpl {
    let operon = operon_ident();
    let mem_storage_ident = mem_storage_ident(service_id);

    parse_quote! {
        impl #mem_storage_ident {
            fn new(_options: #operon::options::MemStorageOptions) -> Self {
                Self::default()
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
    fn test_impl_from_options(service_id: syn::Ident) {
        let item = impl_new(&service_id);
        assert_item_eq(&item, "storage/mem/impl_new.rs");
    }
}
