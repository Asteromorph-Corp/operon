use indexmap::IndexMap;

use super::config_decl::ConfigDecl;
use crate::AllConfig;

pub fn parse_config(input: proc_macro::TokenStream) -> syn::Result<AllConfig> {
    let config_decl: ConfigDecl = syn::parse(input)?;
    let service_id = config_decl.service_id.to_string();
    let primary_entity = config_decl.primary_entity.id.to_string();
    let primary_dimension = config_decl
        .primary_entity
        .dims
        .first()
        .map(|d| d.to_string())
        .ok_or_else(|| {
            syn::Error::new(
                config_decl.primary_entity._span,
                "Primary entity must have a dimension",
            )
        })?;

    Ok(AllConfig {
        service_id,
        primary_entity,
        primary_dimension,
        // TODO: Parse these fields properly
        dimensions: IndexMap::new(),
        entities: IndexMap::new(),
        jobs: IndexMap::new(),
    })
}
