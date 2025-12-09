use std::collections::HashSet;

use indexmap::IndexMap;
use syn::parse::{Parse, ParseBuffer};

use super::config_decl::ConfigDecl;
use crate::configs::{
    AllConfig, DimensionConfig, DimensionConfigMap, EntityConfig, EntityConfigMap, JobArg,
    JobConfig, JobConfigMap,
};

fn validate_downwards_closed<'a>(
    dims: impl IntoIterator<Item = &'a syn::Ident>,
    configs: &DimensionConfigMap,
) -> Result<(), String> {
    let dims_set: HashSet<&syn::Ident> = HashSet::from_iter(dims);
    for dim in &dims_set {
        let Some(config) = configs.get(*dim) else {
            return Err(format!("Dimension '{dim}' is not defined"));
        };
        for dep in &config.depends_on {
            if !dims_set.contains(dep) {
                return Err(format!(
                    "Cannot iterate over '{dim}' without iterating over '{dep}'"
                ));
            }
        }
    }
    Ok(())
}

impl Parse for AllConfig {
    fn parse(input: &ParseBuffer) -> syn::Result<AllConfig> {
        let config_decl: ConfigDecl = input.parse()?;
        let service_id = config_decl.service_id;
        let mut dimensions: DimensionConfigMap = IndexMap::new();
        let mut entities: EntityConfigMap = IndexMap::new();
        let mut jobs: JobConfigMap = IndexMap::new();

        for job in config_decl.jobs {
            let new_entity = job.spawned_entity;
            let args = job.args;
            let pool = job
                .pool
                .map(|lit| {
                    lit.base10_parse::<usize>().map_err(|_| {
                        syn::Error::new(lit.span(), format!("Invalid pool value: '{lit}'"))
                    })
                })
                .transpose()?
                .unwrap_or(1);
            let dims = job.dims;

            // Deduplicate and verify
            // Constraint 1: Defining entity must not conflict with existing entities
            if entities.contains_key(&new_entity.id) {
                return Err(syn::Error::new(
                    new_entity._span,
                    format!("Entity '{}' is already defined", new_entity.id),
                ));
            }
            // Constraint 2: Spawned dimension must not conflict with existing dimensions
            if let Some(new_dim) = new_entity.dims.first()
                && dimensions.contains_key(new_dim)
            {
                return Err(syn::Error::new(
                    new_entity._span,
                    format!("Cannot define dimension '{new_dim}' again"),
                ));
            }
            // Constraint 3: Job name must be unique as a snake_case identifier
            if jobs.contains_key(&job.id) {
                return Err(syn::Error::new(
                    job._span,
                    format!("Job '{}' is already defined", job.id),
                ));
            }
            // Constraint 4: Arguments must be already-defined, valid entities
            for arg_entity in &args {
                // Constraint 4a: Argument entity must be defined
                let Some(arg_entity_config) = entities.get(&arg_entity.id) else {
                    return Err(syn::Error::new(
                        arg_entity._span,
                        format!("Undefined entity '{}'", arg_entity.id),
                    ));
                };
                // Constraint 4b–d. Let A = arg_entity_config.dims, B = arg_entity.dims, C =
                // job.dims. Use HashSet for set operations.
                let a_set: HashSet<&syn::Ident> = HashSet::from_iter(&arg_entity_config.dims);
                let b_set: HashSet<&syn::Ident> = HashSet::from_iter(&arg_entity.dims);
                // Constraint 4b: B ⊆ A.
                for dim in &arg_entity.dims {
                    if !a_set.contains(dim) {
                        return Err(syn::Error::new(
                            dim.span(),
                            format!(
                                "Dimension '{dim}' is not part of entity '{}'",
                                arg_entity.id
                            ),
                        ));
                    }
                }

                // Constraint 4d: A \ B must be downwards closed.
                if let Err(err) =
                    validate_downwards_closed(a_set.difference(&b_set).cloned(), &dimensions)
                {
                    return Err(syn::Error::new(arg_entity._span, err));
                }
            }
            // Constraint 5: Dimensions must be predefined and downwards closed.
            for dim in &dims {
                if !dimensions.contains_key(dim) {
                    return Err(syn::Error::new(
                        dim.span(),
                        format!("Dimension '{dim}' is not defined"),
                    ));
                }
            }
            // Constraint 6: \mathcal{F} = \bigcup_i \Sigma(\tau_{in,i}) \setminus
            // \mathcal{E}_{in,i}
            let bigcup = args
                .iter()
                .map(|arg| -> syn::Result<_> {
                    let Some(arg_config) = entities.get(&arg.id) else {
                        return Err(syn::Error::new(
                            arg._span,
                            format!("Undefined entity '{}'", arg.id),
                        ));
                    };
                    let a_set: HashSet<&syn::Ident> = HashSet::from_iter(&arg_config.dims);
                    let b_set: HashSet<&syn::Ident> = HashSet::from_iter(&arg.dims);
                    Ok(a_set.difference(&b_set).cloned().collect::<Vec<_>>())
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten()
                .collect::<HashSet<_>>();
            let job_dims: HashSet<&syn::Ident> = HashSet::from_iter(&dims);

            if job_dims != bigcup {
                let expected = bigcup.iter().map(|d| d.to_string()).collect::<Vec<_>>();
                let err_msg = if expected.is_empty() {
                    "No dimensions expected".to_string()
                } else {
                    format!("Dimensions do not match, should be: {}", expected.join(","))
                };
                return Err(syn::Error::new(job._span, err_msg));
            }

            if let Err(err) = validate_downwards_closed(&dims, &dimensions) {
                return Err(syn::Error::new(job._span, err));
            }

            // Add the new configs
            let new_entity_dims = dims
                .iter()
                .chain(new_entity.dims.iter())
                .cloned()
                .collect::<Vec<_>>();
            let new_entity_config = EntityConfig {
                id: new_entity.id.clone(),
                dims: new_entity_dims,
            };
            entities.insert(new_entity.id.clone(), new_entity_config);
            if let Some(dim) = new_entity.dims.first() {
                let new_dim_config = DimensionConfig {
                    id: dim.clone(),
                    depends_on: dims.clone(),
                };
                dimensions.insert(dim.clone(), new_dim_config);
            }
            let job_config = JobConfig {
                id: job.id.clone(),
                from: args
                    .into_iter()
                    .map(|e| JobArg {
                        id: e.id,
                        over: e.dims,
                    })
                    .collect::<Vec<_>>(),
                to: new_entity.id,
                dims: dims.clone(),
                spawn_dim: new_entity.dims.first().cloned(),
                pool_size: pool,
            };
            jobs.insert(job.id, job_config);
        }

        Ok(AllConfig {
            service_id,
            dimensions,
            entities,
            jobs,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let input = "splitter = {
                Input<input_no> = get_inputs();
                Intermediate<word_no> = get_words(Input) for input_no;
                Output<char_no> = get_chars(Intermediate) for input_no, word_no;
            }";
        let parsed: AllConfig = syn::parse_str(input).expect("Failed to parse config");
        assert_eq!(parsed.service_id, "splitter");
    }
}
