use quote::quote;
use syn::parse_quote;

use crate::configs::{DimensionConfigMap, DimensionId};
use crate::macros::schema::resolution::impl_enum_from_resolution::impl_enum_from_resolution;
use crate::macros::schema::resolution::impl_resolution::impl_resolution;
use crate::macros::schema::resolution::impl_resolution_enum::impl_resolution_enum;
use crate::macros::schema::resolution::impl_resolution_sql::impl_resolution_sql;
use crate::macros::schema::resolution::resolution_definition::resolution_definition;
use crate::macros::schema::resolution::resolution_enum::resolution_enum;

/// Generates the `mod resolution` module with all resolution-related items.
pub fn mod_resolution(
    primary_dimension: &DimensionId,
    dimensions: &DimensionConfigMap,
) -> syn::ItemMod {
    let resolution_enum = resolution_enum(dimensions);
    let impl_resolution_enum = impl_resolution_enum(primary_dimension);

    let resolutions = dimensions.values().map(|dimension| {
        let def = resolution_definition(dimension);
        let impl_resolution = impl_resolution(dimension);
        let impl_resolution_sql = impl_resolution_sql(dimension);
        let impl_enum_from_resolution = impl_enum_from_resolution(dimension);

        quote! {
            #def
            #impl_resolution
            #impl_resolution_sql
            #impl_enum_from_resolution
        }
    });

    parse_quote! {
        mod resolution {
            use super::*;

            #resolution_enum
            #impl_resolution_enum

            #(#resolutions)*

        }
    }
}
