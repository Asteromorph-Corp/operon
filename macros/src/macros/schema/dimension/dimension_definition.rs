use syn::parse_quote;

use crate::{configs::DimensionId, utils::dimension_ident};

pub(super) fn dimension_definition(dimension_id: &DimensionId) -> syn::ItemType {
    let dim_ident = dimension_ident(dimension_id);

    parse_quote! {
        pub type #dim_ident = usize;
    }
}
