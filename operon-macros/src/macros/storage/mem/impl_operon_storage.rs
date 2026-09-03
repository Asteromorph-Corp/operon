use syn::parse_quote;

use crate::operon_ident;
use crate::utils::mem_storage_ident;

/// Generates an implementation of the `OperonStorage` trait for a given service ID.
///
/// # Example
/// ```rust,ignore
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/storage/mem/impl_operon_storage.rs") )]
/// ```
pub(super) fn impl_operon_storage(service_id: &syn::Ident) -> syn::ItemImpl {
    let operon = operon_ident();
    let mem_storage_ident = mem_storage_ident(service_id);

    parse_quote! {
        #[#operon::__private::async_trait::async_trait]
        impl #operon::OperonStorage for #mem_storage_ident {
            type Error = std::convert::Infallible;

            async fn init(&self) -> #operon::error::StorageResult<(), Self::Error> {
                Ok(())
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
    fn test_impl_operon_storage(service_id: syn::Ident) {
        let item = impl_operon_storage(&service_id);
        assert_item_eq(&item, "storage/mem/impl_operon_storage.rs");
    }
}
