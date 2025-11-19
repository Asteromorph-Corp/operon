use syn::parse_quote;

use crate::configs::DimensionConfigMap;
use crate::macros::schema::resolution::impl_resolution_enum::impl_resolution_enum;
use crate::macros::schema::resolution::resolution_enum::resolution_enum;

/// Generates the `mod resolution` module with all resolution-related items.
pub fn mod_resolution(dimensions: &DimensionConfigMap) -> syn::ItemMod {
    let resolution_enum = resolution_enum(dimensions);
    let impl_resolution_enum = impl_resolution_enum();

    parse_quote! {
        mod resolution {
            use super::*;

            #resolution_enum
            #impl_resolution_enum
        }
    }
}
