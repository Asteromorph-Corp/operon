use std::collections::HashSet;
use std::hash::RandomState;

use heck::{ToPascalCase, ToSnakeCase};
use indexmap::IndexMap;
use syn::parse::{Parse, ParseBuffer};

use super::config_decl::ConfigDecl;
use crate::configs::{
    AllConfig, DimensionConfig, DimensionConfigMap, EntityConfig, EntityConfigMap, JobArg,
    JobConfig, JobConfigMap,
};

fn validate_downwards_closed(dims: &[String], configs: &DimensionConfigMap) -> Result<(), String> {
    let dims_set: HashSet<_, RandomState> = HashSet::from_iter(dims.iter());
    for dim in dims {
        let Some(config) = configs.get(dim) else {
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
        let service_id = config_decl.service_id.to_string();
        let mut dimensions: DimensionConfigMap = IndexMap::new();
        let mut entities: EntityConfigMap = IndexMap::new();
        let mut jobs: JobConfigMap = IndexMap::new();

        for job in config_decl.jobs {
            let new_entity = job.spawned_entity;
            let job_id = job.id.to_string().to_snake_case();
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
            if entities.contains_key(&new_entity.id.to_string()) {
                return Err(syn::Error::new(
                    new_entity._span,
                    format!("Entity '{}' is already defined", new_entity.id),
                ));
            }
            // Constraint 2: Spawned dimension must not conflict with existing dimensions
            if let Some(new_dim) = new_entity.dims.first()
                && dimensions.contains_key(&new_dim.to_string())
            {
                return Err(syn::Error::new(
                    new_entity._span,
                    format!("Cannot define dimension '{new_dim}' again"),
                ));
            }
            // Constraint 3: Job name must be unique as a snake_case identifier
            if jobs.contains_key(&job_id) {
                return Err(syn::Error::new(
                    job._span,
                    format!("Job '{job_id}' is already defined"),
                ));
            }
            // Constraint 4: Arguments must be already-defined, valid entities
            for arg_entity in &args {
                let arg_entity_id = arg_entity.id.to_string().to_pascal_case();
                // Constraint 4a: Argument entity must be defined
                let Some(arg_entity_config) = entities.get(&arg_entity_id) else {
                    return Err(syn::Error::new(
                        arg_entity._span,
                        format!("Undefined entity '{arg_entity_id}'"),
                    ));
                };
                // Constraint 4b–d. Let A = arg_entity_config.dims, B = arg_entity.dims, C =
                // job.dims. Use HashSet for set operations.
                let a_set: HashSet<String, RandomState> =
                    HashSet::from_iter(arg_entity_config.dims.iter().cloned());
                let b_set: HashSet<String, RandomState> = HashSet::from_iter(
                    arg_entity
                        .dims
                        .iter()
                        .map(|d| d.to_string().to_snake_case()),
                );
                // Constraint 4b: B ⊆ A.
                for dim in &arg_entity.dims {
                    if !a_set.contains(&dim.to_string().to_snake_case()) {
                        return Err(syn::Error::new(
                            dim.span(),
                            format!("Dimension '{dim}' is not part of entity '{arg_entity_id}'"),
                        ));
                    }
                }
                // Constraint 4d: A \ B must be downwards closed.
                if let Err(err) = validate_downwards_closed(
                    &a_set
                        .difference(&b_set)
                        .map(|d| d.to_string())
                        .collect::<Vec<_>>(),
                    &dimensions,
                ) {
                    return Err(syn::Error::new(arg_entity._span, err));
                }
            }
            // Constraint 5: Dimensions must be predefined and downwards closed.
            for dim in &dims {
                let dim_id = dim.to_string().to_snake_case();
                if !dimensions.contains_key(&dim_id) {
                    return Err(syn::Error::new(
                        dim.span(),
                        format!("Dimension '{dim_id}' is not defined"),
                    ));
                }
            }
            // Constraint 6: \mathcal{F} = \bigcup_i \Sigma(\tau_{in,i}) \setminus
            // \mathcal{E}_{in,i}
            let bigcup = args
                .iter()
                .map(|arg| -> syn::Result<_> {
                    let arg_id = arg.id.to_string().to_pascal_case();
                    let Some(arg_config) = entities.get(&arg_id) else {
                        return Err(syn::Error::new(
                            arg._span,
                            format!("Undefined entity '{arg_id}'"),
                        ));
                    };
                    let a_set: HashSet<String> =
                        HashSet::from_iter(arg_config.dims.iter().cloned());
                    let b_set: HashSet<String> =
                        HashSet::from_iter(arg.dims.iter().map(|d| d.to_string().to_snake_case()));

                    Ok(a_set.difference(&b_set).cloned().collect::<Vec<_>>())
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten()
                .collect::<HashSet<_>>();
            let job_dims = HashSet::from_iter(dims.iter().map(|d| d.to_string().to_snake_case()));

            if job_dims != bigcup {
                let expected = bigcup.iter().map(String::as_str).collect::<Vec<_>>();
                let err_msg = if expected.is_empty() {
                    "No dimensions expected".to_string()
                } else {
                    format!("Dimensions do not match, should be: {}", expected.join(","))
                };
                return Err(syn::Error::new(job._span, err_msg));
            }

            if let Err(err) = validate_downwards_closed(
                &dims
                    .iter()
                    .map(|d| d.to_string().to_snake_case())
                    .collect::<Vec<_>>(),
                &dimensions,
            ) {
                return Err(syn::Error::new(job._span, err));
            }

            // Add the new configs
            let new_entity_id = new_entity.id.to_string().to_pascal_case();
            let new_entity_dims = dims
                .iter()
                .chain(new_entity.dims.iter())
                .map(|d| d.to_string().to_snake_case())
                .collect::<Vec<_>>();
            let new_entity_config = EntityConfig {
                id: new_entity_id.clone(),
                dims: new_entity_dims,
            };
            entities.insert(new_entity_id.clone(), new_entity_config);
            if let Some(dim) = new_entity.dims.first() {
                let new_dim_id = dim.to_string().to_snake_case();
                let new_dim_config = DimensionConfig {
                    id: new_dim_id.clone(),
                    depends_on: dims
                        .iter()
                        .map(|d| d.to_string().to_snake_case())
                        .collect::<Vec<_>>(),
                };
                dimensions.insert(new_dim_id, new_dim_config);
            }
            let job_config = JobConfig {
                id: job_id.clone(),
                from: args
                    .iter()
                    .map(|e| JobArg {
                        id: e.id.to_string().to_pascal_case(),
                        over: e
                            .dims
                            .iter()
                            .map(|d| d.to_string().to_snake_case())
                            .collect::<Vec<_>>(),
                    })
                    .collect::<Vec<_>>(),
                to: new_entity_id,
                dims: dims
                    .iter()
                    .map(|d| d.to_string().to_snake_case())
                    .collect::<Vec<_>>(),
                spawn_dim: new_entity
                    .dims
                    .first()
                    .map(|d| d.to_string().to_snake_case()),
                pool_size: pool,
            };
            jobs.insert(job_id, job_config);
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
