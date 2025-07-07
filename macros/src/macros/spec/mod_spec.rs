use syn::parse_quote;

use crate::{
    AllConfig,
    macros::spec::primary::{impl_primary_spec, primary_spec_definition},
};

pub fn mod_spec(all_configs: &AllConfig) -> syn::ItemMod {
    let primary_spec = primary_spec_definition();
    let impl_primary_spec = impl_primary_spec(
        &all_configs.service_id,
        &all_configs.primary_entity,
        &all_configs.primary_dimension,
    );

    parse_quote! {
        mod spec {
            use super::*;

            #primary_spec
            #impl_primary_spec



            // Add your spec-related items here
        }
    }
}
