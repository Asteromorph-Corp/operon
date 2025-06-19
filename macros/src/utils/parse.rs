use super::config_types::*;
use indexmap::IndexMap;
use proc_macro_error::OptionExt;
use quote::{ToTokens, quote};
use std::path::PathBuf;
use syn::{
    ExprAssign, Item, LitInt, LitStr, Meta, Token, braced, bracketed,
    parse::{Parse, ParseBuffer, ParseStream},
    parse2,
    spanned::Spanned,
};

fn check_valid_name(name: &str) -> syn::Result<()> {
    if name.chars().any(|c| !c.is_alphanumeric() && c != '_') {
        return Err(syn::Error::new_spanned(
            name,
            "Invalid name, only alphanumeric characters and underscores are allowed",
        ));
    }
    Ok(())
}

fn check_valid_namedef(namedef: &str) -> syn::Result<()> {
    if namedef
        .chars()
        .any(|c| !c.is_alphanumeric() && !matches!(c, '_' | '|' | ',' | ' '))
    {
        return Err(syn::Error::new_spanned(
            namedef,
            "Invalid definition, syntax: `name` or `name|dim1,dim2,...`",
        ));
    }
    let parts: Vec<&str> = namedef.split('|').collect();
    if parts.len() > 2 {
        return Err(syn::Error::new_spanned(
            namedef,
            "Invalid definition, syntax: `name` or `name|dim1,dim2,...`",
        ));
    }
    check_valid_name(parts[0].trim())?;
    if parts.len() == 1 {
        return Ok(());
    }
    let dims: Vec<&str> = parts[1].split(',').collect();
    for dim in dims {
        if dim.trim().is_empty() {
            return Err(syn::Error::new_spanned(
                dim,
                "Invalid definition, dimensions cannot be empty",
            ));
        }
        check_valid_name(dim.trim())?;
    }
    Ok(())
}

#[derive(Debug, Default)]
struct RawEntityAttrs {
    primary: bool,
    dims: Vec<String>,
    def: Option<String>,
    from: Option<Vec<String>>,
    pool: Option<usize>,
}
impl Parse for RawEntityAttrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attrs = RawEntityAttrs::default();
        while !input.is_empty() {
            let arg_name = input.parse::<syn::Ident>()?;
            match arg_name.to_string().as_str() {
                "primary" => {
                    if attrs.primary {
                        return Err(syn::Error::new_spanned(
                            arg_name,
                            "Only the first entity can be primary",
                        ));
                    }
                    attrs.primary = true;
                }
                "dims" => {
                    let _ = input.parse::<Token![=]>()?;
                    let dims_content;
                    let _ = bracketed!(dims_content in input);
                    attrs.dims = dims_content
                        .parse_terminated(ParseBuffer::parse::<LitStr>, Token![,])?
                        .into_iter()
                        .map(|lit| lit.value())
                        .collect();
                    for dim in &attrs.dims {
                        check_valid_name(dim)?;
                    }
                }
                "def" => {
                    let _ = input.parse::<Token![=]>()?;
                    let def_lit = input.parse::<LitStr>()?;
                    check_valid_namedef(&def_lit.value())?;
                    attrs.def = Some(def_lit.value());
                }
                "from" => {
                    let _ = input.parse::<Token![=]>()?;
                    let from_content;
                    let _ = bracketed!(from_content in input);
                    attrs.from = Some(
                        from_content
                            .parse_terminated(ParseBuffer::parse::<LitStr>, Token![,])?
                            .into_iter()
                            .map(|lit| lit.value())
                            .collect(),
                    );
                    if let Some(from) = &attrs.from {
                        for def in from {
                            check_valid_namedef(def)?;
                        }
                    }
                }
                "pool" => {
                    let _ = input.parse::<Token![=]>()?;
                    let pool_lit = input.parse::<LitInt>()?;
                    let pool = pool_lit.base10_parse()?;
                    if pool < 1 {
                        return Err(syn::Error::new_spanned(
                            pool_lit,
                            "Pool size must be at least 1",
                        ));
                    }
                    attrs.pool = Some(pool);
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        arg_name,
                        "Unknown entity attribute",
                    ));
                }
            }
        }
        Ok(attrs)
    }
}

#[derive(Debug, Default)]
struct RawEntity {
    primary: bool,
    dims: Vec<String>,
    def: Option<String>,
    from: Option<Vec<String>>,
    pool: Option<usize>,
    body: String,
}

#[derive(Debug, Default)]
struct RawTypesConfig {
    entity: IndexMap<String, RawEntity>,
}
impl Parse for RawTypesConfig {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut types = RawTypesConfig::default();
        let mut first = true;
        while !input.is_empty() {
            let mut raw_entity = RawEntity::default();
            let item = input.parse::<Item>()?;
            if let Item::Struct(mut item_struct) = item {
                if item_struct.vis != syn::Visibility::Public(Token![pub](item_struct.vis.span())) {
                    item_struct.vis = syn::Visibility::Public(Token![pub](item_struct.vis.span()));
                }
                let attrs = {
                    let mut attrs: RawEntityAttrs = RawEntityAttrs::default();
                    for attr in &item_struct.attrs {
                        if attr.path().is_ident("entity") {
                            let Meta::List(meta_list) = attr.meta.clone() else {
                                return Err(syn::Error::new_spanned(
                                    attr,
                                    "Expected `entity` attribute to be a list",
                                ));
                            };
                            attrs = parse2(meta_list.tokens)?;
                        } else {
                            return Err(syn::Error::new_spanned(
                                attr,
                                "Unknown attribute, expected `entity`",
                            ));
                        }
                    }
                    attrs
                };
                match (first, attrs.primary) {
                    (true, true) => {
                        first = false;
                    }
                    (false, true) => {
                        return Err(syn::Error::new_spanned(
                            item_struct,
                            "Only the first entity can be primary",
                        ));
                    }
                    (true, false) => {
                        return Err(syn::Error::new_spanned(
                            item_struct,
                            "The first entity must be primary",
                        ));
                    }
                    (false, false) => {}
                }
                if attrs.primary {
                    if !attrs.dims.len() != 1 {
                        return Err(syn::Error::new_spanned(
                            item_struct,
                            "Primary entity must have exactly one dimension",
                        ));
                    }
                    if attrs.def.is_some() {
                        return Err(syn::Error::new_spanned(
                            item_struct,
                            "Primary entity cannot have a definition",
                        ));
                    }
                    if attrs.from.is_some() {
                        return Err(syn::Error::new_spanned(
                            item_struct,
                            "Primary entity cannot depend on other entities",
                        ));
                    }
                    if attrs.pool.is_some() {
                        return Err(syn::Error::new_spanned(
                            item_struct,
                            "Primary entity cannot have a pool size",
                        ));
                    }
                } else {
                    if attrs.def.is_none() {
                        return Err(syn::Error::new_spanned(
                            item_struct,
                            "Non-primary entities must have a definition",
                        ));
                    }
                    if attrs.from.is_none() {
                        return Err(syn::Error::new_spanned(
                            item_struct,
                            "Non-primary entities must have a `from` attribute, \
                            if you meant to signify this entity as a source, \
                            use `from = []`",
                        ));
                    }
                }
                let name = item_struct.ident.to_string();
                check_valid_name(&name)?;
                if types.entity.contains_key(&name) {
                    return Err(syn::Error::new_spanned(
                        item_struct.ident,
                        format!("Entity `{}` is already defined", name),
                    ));
                }
                if !item_struct.generics.params.is_empty() {
                    return Err(syn::Error::new_spanned(
                        item_struct.generics,
                        "Generic paramenters are not allowed",
                    ));
                }
                if item_struct.generics.where_clause.is_some() {
                    return Err(syn::Error::new_spanned(
                        item_struct.generics.where_clause.as_ref().unwrap(),
                        "Where clauses are not allowed",
                    ));
                }
                raw_entity.primary = attrs.primary;
                raw_entity.dims = attrs.dims;
                raw_entity.def = attrs.def;
                raw_entity.from = attrs.from;
                raw_entity.pool = attrs.pool;
                for field in &mut item_struct.fields {
                    if field.vis != syn::Visibility::Public(Token![pub](field.vis.span())) {
                        field.vis = syn::Visibility::Public(Token![pub](field.vis.span()));
                    }
                }
                let fields = item_struct.fields.to_token_stream();
                raw_entity.body = quote! {
                    pub struct #name {
                        #fields
                    }
                }
                .to_string();
                types.entity.insert(name, raw_entity);
            } else if let Item::Enum(mut item_enum) = item {
                if item_enum.vis != syn::Visibility::Public(Token![pub](item_enum.vis.span())) {
                    item_enum.vis = syn::Visibility::Public(Token![pub](item_enum.vis.span()));
                }
                let attrs = {
                    let mut attrs: RawEntityAttrs = RawEntityAttrs::default();
                    for attr in &item_enum.attrs {
                        if attr.path().is_ident("entity") {
                            let Meta::List(meta_list) = attr.meta.clone() else {
                                return Err(syn::Error::new_spanned(
                                    attr,
                                    "Expected `entity` attribute to be a list",
                                ));
                            };
                            attrs = parse2(meta_list.tokens)?;
                        } else {
                            return Err(syn::Error::new_spanned(
                                attr,
                                "Unknown attribute, expected `entity`",
                            ));
                        }
                    }
                    attrs
                };
                match (first, attrs.primary) {
                    (true, true) => {
                        first = false;
                    }
                    (false, true) => {
                        return Err(syn::Error::new_spanned(
                            item_enum,
                            "Only the first entity can be primary",
                        ));
                    }
                    (true, false) => {
                        return Err(syn::Error::new_spanned(
                            item_enum,
                            "The first entity must be primary",
                        ));
                    }
                    (false, false) => {}
                }

                if attrs.primary {
                    if !attrs.dims.len() != 1 {
                        return Err(syn::Error::new_spanned(
                            item_enum,
                            "Primary entity must have exactly one dimension",
                        ));
                    }
                    if attrs.def.is_some() {
                        return Err(syn::Error::new_spanned(
                            item_enum,
                            "Primary entity cannot have a definition",
                        ));
                    }
                    if attrs.from.is_some() {
                        return Err(syn::Error::new_spanned(
                            item_enum,
                            "Primary entity cannot depend on other entities",
                        ));
                    }
                    if attrs.pool.is_some() {
                        return Err(syn::Error::new_spanned(
                            item_enum,
                            "Primary entity cannot have a pool size",
                        ));
                    }
                } else {
                    if attrs.def.is_none() {
                        return Err(syn::Error::new_spanned(
                            item_enum,
                            "Non-primary entities must have a definition",
                        ));
                    }
                    if attrs.from.is_none() {
                        return Err(syn::Error::new_spanned(
                            item_enum,
                            "Non-primary entities must have a `from` attribute, \
                            if you meant to signify this entity as a source, \
                            use `from = []`",
                        ));
                    }
                }
                let name = item_enum.ident.to_string();
                check_valid_name(&name)?;
                if types.entity.contains_key(&name) {
                    return Err(syn::Error::new_spanned(
                        item_enum.ident,
                        format!("Entity `{}` is already defined", name),
                    ));
                }
                if !item_enum.generics.params.is_empty() {
                    return Err(syn::Error::new_spanned(
                        item_enum.generics,
                        "Generic parameters are not allowed",
                    ));
                }
                if item_enum.generics.where_clause.is_some() {
                    return Err(syn::Error::new_spanned(
                        item_enum.generics.where_clause.as_ref().unwrap(),
                        "Where clauses are not allowed",
                    ));
                }
                raw_entity.primary = attrs.primary;
                raw_entity.dims = attrs.dims;
                raw_entity.def = attrs.def;
                raw_entity.from = attrs.from;
                raw_entity.pool = attrs.pool;
                let variants = item_enum.variants.to_token_stream();
                raw_entity.body = quote! {
                    pub enum #name {
                        #variants
                    }
                }
                .to_string();
                types.entity.insert(name, raw_entity);
            } else {
                return Err(syn::Error::new_spanned(
                    item,
                    "Expected an entity struct or enum",
                ));
            }
        }
        Ok(types)
    }
}

#[derive(Debug, Default)]
struct RawLogConfig {
    level: Option<String>,
    buffer_size: Option<usize>,
    dump: Option<bool>,
    dump_path: Option<String>,
}

#[derive(Debug, Default)]
struct RawCommonConfig {
    storage: StorageConfig,
    log: RawLogConfig,
}
impl Parse for RawCommonConfig {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut config = RawCommonConfig::default();
        while !input.is_empty() {
            let arg: ExprAssign = input.parse()?;
            match arg.left.to_token_stream().to_string().as_str().trim() {
                "storage.data.uri" => {
                    config.storage.data.uri = parse2::<LitStr>(arg.right.to_token_stream())?.value();
                }
                "storage.data.schema" => {
                    let schema = parse2::<LitStr>(arg.right.to_token_stream())?.value();
                    if schema.is_empty() {
                        return Err(syn::Error::new_spanned(arg.right, "Schema cannot be empty"));
                    }
                    check_valid_name(&schema)?;
                    config.storage.data.schema = Some(schema);
                }
                "storage.data.pool_size" => {
                    config.storage.data.pool_size = arg
                        .right
                        .to_token_stream()
                        .to_string()
                        .parse()
                        .unwrap_or(16);
                    if config.storage.data.pool_size < 1 {
                        return Err(syn::Error::new_spanned(
                            arg.right,
                            "Pool size must be at least 1",
                        ));
                    }
                }
                "storage.metadata.uri" => {
                    config.storage.metadata.uri = parse2::<LitStr>(arg.right.to_token_stream())?.value();
                }
                "storage.metadata.schema" => {
                    let schema = parse2::<LitStr>(arg.right.to_token_stream())?.value();
                    if schema.is_empty() {
                        return Err(syn::Error::new_spanned(arg.right, "Schema cannot be empty"));
                    }
                    check_valid_name(&schema)?;
                    config.storage.metadata.schema = Some(schema);
                }
                "storage.metadata.pool_size" => {
                    config.storage.metadata.pool_size = arg
                        .right
                        .to_token_stream()
                        .to_string()
                        .parse()
                        .unwrap_or(16);
                    if config.storage.metadata.pool_size < 1 {
                        return Err(syn::Error::new_spanned(
                            arg.right,
                            "Pool size must be at least 1",
                        ));
                    }
                }
                "log.level" => {
                    let level = parse2::<LitStr>(arg.right.to_token_stream())?.value();
                    if !matches!(
                        level.as_str(),
                        "trace" | "debug" | "info" | "warn" | "error"
                    ) {
                        return Err(syn::Error::new_spanned(
                            arg.right,
                            "Invalid log level, must be one of: trace, debug, info, warn, error",
                        ));
                    }
                    config.log.level = Some(level);
                }
                "log.buffer_size" => {
                    config.log.buffer_size =
                        Some(parse2::<LitInt>(arg.right.to_token_stream())?.base10_parse()?);
                }
                "log.dump" => {
                    config.log.dump =
                        Some(arg.right.to_token_stream().to_string().parse().unwrap());
                }
                "log.dump_path" => {
                    config.log.dump_path = Some(parse2::<LitStr>(arg.right.to_token_stream())?.value());
                }
                _ => return Err(syn::Error::new_spanned(arg, "Unknown configuration option")),
            }
            let _ = input.parse::<Token![,]>();
        }
        Ok(config)
    }
}

#[derive(Debug, Default)]
struct RawAllConfig {
    types: RawTypesConfig,
    common: RawCommonConfig,
    use_psql_storage: Option<String>,
}
impl Parse for RawAllConfig {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut all_configs = RawAllConfig::default();
        while !input.is_empty() {
            let ident = input.parse::<syn::Ident>()?;
            let _ = input.parse::<Token![=]>();
            match ident.to_string().as_str() {
                "config" => {
                    let content;
                    let _ = braced!(content in input);
                    all_configs.common = content.parse()?;
                }
                "types" => {
                    let content;
                    let _ = braced!(content in input);
                    all_configs.types = content.parse()?;
                }
                "use_psql_storage" => {
                    all_configs.use_psql_storage = Some(input.parse::<syn::Ident>()?.to_string());
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        ident,
                        "Unknown configuration section",
                    ));
                }
            }
            let _ = input.parse::<Token![;]>();
        }
        Ok(all_configs)
    }
}

fn resolve_common_config(raw: RawCommonConfig) -> syn::Result<CommonConfig> {
    let storage = StorageConfig {
        metadata: Connection {
            uri: raw.storage.metadata.uri,
            schema: raw.storage.metadata.schema,
            pool_size: raw.storage.metadata.pool_size,
        },
        data: Connection {
            uri: raw.storage.data.uri,
            schema: raw.storage.data.schema,
            pool_size: raw.storage.data.pool_size,
        },
    };
    let log = LogConfig {
        level: raw.log.level.unwrap_or_else(|| "info".to_string()),
        buffer_size: raw.log.buffer_size.unwrap_or(1024),
        dump: raw.log.dump.unwrap_or(false),
        dump_path: match std::env::var("CARGO_MANIFEST_DIR") {
            Ok(manifest_dir) => {
                let mut path = PathBuf::from(manifest_dir);
                path.push(raw.log.dump_path.unwrap_or_else(|| "logs".to_string()));
                path
            }
            Err(_) => raw
                .log
                .dump_path
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("logs")),
        },
    };
    Ok(CommonConfig { storage, log })
}

fn resolve_types_config(raw: RawTypesConfig) -> syn::Result<TypesConfig> {
    let mut entities: Entities = Entities::new();
    let mut dimensions: Dimensions = Dimensions::new();
    let mut jobs: Jobs = Jobs::new();
    for (i, (name, raw_entity)) in raw.entity.iter().enumerate() {
        // raw_entity.primary
        let primary = raw_entity.primary;
        // raw_entity.dims
        let entity_dims_raw = &raw_entity.dims;
        let (entity_dims, entity_def, entity_from) = if !primary {
            let mut spawned_dim = None;
            let mut entity_dims = vec![];
            for dim in entity_dims_raw {
                let maybe_dim = Dimension::get_by_name(&dimensions, dim);
                if let Some(dim) = maybe_dim {
                    entity_dims.push(dim.tag);
                } else if spawned_dim.is_none() {
                    let new_dimension = Dimension {
                        tag: DimensionTag(dimensions.len()),
                        name: dim.clone(),
                        primary: false,
                        depends_on: vec![],
                    };
                    spawned_dim = Some(new_dimension.tag);
                    entity_dims.push(new_dimension.tag);
                    dimensions.push(new_dimension);
                } else {
                    panic!("Tried to define multiple dimensions for entity `{name}`");
                }
            }
            let Some(def_raw) = &raw_entity.def else {
                panic!("Non-primary entity `{name}` must have a definition");
            };
            let (job_name, job_dims) = match def_raw.split('|').collect::<Vec<&str>>().as_slice() {
                [def_name] => (def_name.trim().to_string(), vec![]),
                [def_name, dims] => {
                    let dims = dims
                        .split(',')
                        .map(|d| {
                            Dimension::get_by_name(&dimensions, d.trim())
                                .expect_or_abort(&format!(
                                    "Dimension {} used before defined",
                                    d.trim()
                                ))
                                .tag
                        })
                        .collect::<Vec<_>>();
                    (def_name.trim().to_string(), dims)
                }
                _ => {
                    panic!(
                        "Invalid definition for entity `{name}`: `{def_raw}`. \
                        Expected format: `name|dim1,dim2,...` or `name`"
                    );
                }
            };
            let from = match &raw_entity.from {
                Some(from) => from
                    .iter()
                    .map(
                        |def| match def.split('|').collect::<Vec<&str>>().as_slice() {
                            [name] => (
                                Entity::get_by_name(&entities, name.trim())
                                    .expect_or_abort(&format!(
                                        "Entity {} used before defined",
                                        name.trim()
                                    ))
                                    .tag,
                                vec![],
                            ),
                            [name, dims] => {
                                let from_entity =
                                    Entity::get_by_name(&entities, name.trim()).expect_or_abort(
                                        &format!("Entity {} used before defined", name.trim()),
                                    );
                                let dims = dims
                                    .split(',')
                                    .map(|d| {
                                        Dimension::get_by_name(&dimensions, d.trim())
                                            .expect_or_abort(&format!(
                                                "Dimension {} used before defined",
                                                d.trim()
                                            ))
                                            .tag
                                    })
                                    .inspect(|dim| {
                                        if !from_entity.dims.contains(dim) {
                                            panic!(
                                                "Dimension #{} not found in entity {}",
                                                dim.0, from_entity.name
                                            );
                                        }
                                    })
                                    .collect::<Vec<_>>();
                                (from_entity.tag, dims)
                            }
                            _ => panic!(
                                "Invalid `from` definition for entity `{name}`: `{def}`. \
                                Expected format: `name|dim1,dim2,...` or `name`"
                            ),
                        },
                    )
                    .collect::<Vec<_>>(),
                _ => {
                    if entity_dims_raw.is_empty() {
                        panic!(
                            "Non-primary entity `{name}` must have a `from` attribute or dimensions"
                        );
                    }
                    vec![]
                }
            };
            let pool = raw_entity.pool.unwrap_or(1);

            let job_defines_dims = entity_dims
                .iter()
                .filter(|dim| !job_dims.contains(dim))
                .cloned()
                .collect::<Vec<_>>();

            let new_job = Job {
                tag: JobTag(jobs.len()),
                name: job_name.clone(),
                repeats_on: job_dims,
                from: from.clone(),
                defines: (EntityTag(i), job_defines_dims),
                spawns: spawned_dim,
                pool,
            };
            jobs.push(new_job);

            (
                entity_dims,
                Some(JobTag(i - 1)),
                Some(from.iter().map(|(tag, _)| *tag).collect()),
            )
        } else {
            let new_dimension = Dimension {
                tag: DimensionTag(0),
                name: entity_dims_raw[0].clone(),
                primary: true,
                depends_on: vec![],
            };
            dimensions.push(new_dimension);
            (vec![DimensionTag(0)], None, None)
        };
        let entity = Entity {
            tag: EntityTag(i),
            name: name.clone(),
            primary,
            dims: entity_dims,
            def: entity_def,
            from: entity_from,
            body: raw_entity.body.clone(),
        };
        entities.push(entity);
    }
    Ok(TypesConfig {
        entities,
        dimensions,
        jobs,
    })
}

pub fn get_config(input: proc_macro::TokenStream) -> syn::Result<AllConfig> {
    let raw: RawAllConfig = syn::parse(input)?;
    let types = resolve_types_config(raw.types)?;
    let common = resolve_common_config(raw.common)?;
    let use_psql_storage = raw.use_psql_storage;
    Ok(AllConfig {
        common,
        types,
        use_psql_storage,
    })
}
