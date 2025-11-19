use syn::parse_quote;

use crate::configs::DimensionConfig;
use crate::operon_ident;
use crate::utils::dimension_metadata_ident;

pub fn dimension_metadata(dimension: &DimensionConfig) -> syn::ItemFn {
    let operon = operon_ident();
    let fn_name = dimension_metadata_ident(&dimension.id);

    let n = dimension.depends_on.len();
    let id = &dimension.id;
    let deps = &dimension.depends_on;

    parse_quote! {
        pub const fn #fn_name() -> #operon::schema_base::DimensionMetadata<#n> {
            #operon::schema_base::DimensionMetadata {
                id: #id,
                deps: [#(#deps),*],
            }
        }
    }
}
