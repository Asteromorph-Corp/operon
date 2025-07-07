use syn::parse_quote;

use crate::{
    configs::DimensionConfigMap,
    macros::schema::dimension::dimension_definition::dimension_definition,
};

/// Generates the `mod dimension` module with all dimension-related items.
pub fn mod_dimension(dims: &DimensionConfigMap) -> syn::ItemMod {
    let dimension_defs = dims.keys().map(dimension_definition);

    parse_quote! {
        mod dimension {
            #(#dimension_defs)*
        }
    }
}
