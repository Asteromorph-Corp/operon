use syn::parse_quote;

use crate::{
    configs::DimensionConfigMap,
    macros::schema::dimension::dimension_definition::dimension_definition,
};

pub fn mod_dimension(dims: &DimensionConfigMap) -> syn::ItemMod {
    let dimension_defs = dims.keys().map(dimension_definition);

    parse_quote! {
        mod dimension {
            #(#dimension_defs)*
        }
    }
}
