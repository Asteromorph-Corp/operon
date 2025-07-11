use syn::parse_quote;

use crate::{
    AllConfig,
    utils::{
        entity_ident, entity_over_dim_ident, job_enum_ident, job_fn_ident, operon_ident,
        resolution_enum_ident, service_trait_ident,
    },
};

/// Generates a trait for the service based on the provided AllConfig.
///
/// Example:
/// ```rust,ignore
/// #[operon::async_trait::async_trait]
/// #[automatically_derived]
/// pub trait CookingService:
///     operon::service::OperonService<JobEnum = schema::JobEnum, ResolutionEnum = schema::ResolutionEnum>
/// {
///     async fn beta(&self, a: A,) -> Result<Vec<B>, Box<dyn std::error::Error + Send + Sync>>;
///     async fn gamma(&self, a: A,) -> Result<Vec<C>, Box<dyn std::error::Error + Send + Sync>>;
///     async fn delta(&self, a: A,b: B, c: C,) -> Result<D, Box<dyn std::error::Error + Send + Sync>>;
///     async fn epsilon(&self, b_j: Vec<B>, d_j: Vec<D>,) -> Result<E, Box<dyn std::error::Error + Send + Sync>>;
///     async fn zeta(&self, c_k: Vec<C>, e_k: Vec<E>,) -> Result<F, Box<dyn std::error::Error + Send + Sync>>;
/// }
/// ```
pub fn trait_service(all_configs: &AllConfig) -> syn::ItemTrait {
    let operon = operon_ident();
    let job_enum_ident = job_enum_ident();
    let res_enum_ident = resolution_enum_ident();
    let svc_ident = service_trait_ident(&all_configs.service_id);

    let job_fns = all_configs.jobs.values().map(|job| -> syn::TraitItemFn {
        let fn_name = job_fn_ident(&job.id);
        let args = job.from.iter().map(|arg| -> syn::FnArg {
            let arg_ident = entity_over_dim_ident(&arg.id, &arg.over);
            let entity_ident = entity_ident(&arg.id);
            let ty: syn::Type = arg.over.iter().fold(
                parse_quote! { #entity_ident },
                |acc, _| parse_quote! { Vec<#acc> },
            );

            parse_quote! { #arg_ident: #ty}
        });
        let entity_ident = entity_ident(&job.to);
        let return_ty: syn::Type = job.spawn_dim.as_ref().map_or_else(
            || parse_quote! { #entity_ident },
            |_| parse_quote! { Vec<#entity_ident> },
        );

        parse_quote! {
            async fn #fn_name(&self, #(#args,)*) -> Result<#return_ty, Box<dyn std::error::Error + Send + Sync>>;
        }
    });

    parse_quote! {
        #[#operon::async_trait::async_trait]
        #[automatically_derived]
        pub trait #svc_ident: #operon::service::OperonService<JobEnum = schema::#job_enum_ident, ResolutionEnum = schema::#res_enum_ident> {
            #(#job_fns)*
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        JobArg, JobConfig, JobConfigMap,
        configs::{DimensionConfigMap, EntityConfigMap},
    };

    use super::*;

    #[test]
    fn test_trait_service() {
        let all_configs = AllConfig {
            service_id: "Cooking".to_string(),
            primary_dimension: "i".to_string(),
            primary_entity: "a".to_string(),
            dimensions: DimensionConfigMap::new(),
            entities: EntityConfigMap::new(),
            jobs: JobConfigMap::from_iter([
                (
                    "beta".to_string(),
                    JobConfig {
                        id: "beta".to_string(),
                        from: vec![JobArg {
                            id: "a".to_string(),
                            over: vec![],
                        }],
                        to: "b".to_string(),
                        dims: vec!["i".to_string()],
                        spawn_dim: Some("j".to_string()),
                    },
                ),
                (
                    "gamma".to_string(),
                    JobConfig {
                        id: "gamma".to_string(),
                        from: vec![JobArg {
                            id: "a".to_string(),
                            over: vec![],
                        }],
                        to: "c".to_string(),
                        dims: vec!["i".to_string()],
                        spawn_dim: Some("k".to_string()),
                    },
                ),
                (
                    "delta".to_string(),
                    JobConfig {
                        id: "delta".to_string(),
                        from: vec![
                            JobArg {
                                id: "a".to_string(),
                                over: vec![],
                            },
                            JobArg {
                                id: "b".to_string(),
                                over: vec![],
                            },
                            JobArg {
                                id: "c".to_string(),
                                over: vec![],
                            },
                        ],
                        to: "d".to_string(),
                        dims: vec!["i".to_string(), "j".to_string(), "k".to_string()],
                        spawn_dim: None,
                    },
                ),
                (
                    "epsilon".to_string(),
                    JobConfig {
                        id: "epsilon".to_string(),
                        from: vec![
                            JobArg {
                                id: "b".to_string(),
                                over: vec!["j".to_string()],
                            },
                            JobArg {
                                id: "d".to_string(),
                                over: vec!["j".to_string()],
                            },
                        ],
                        to: "e".to_string(),
                        dims: vec!["i".to_string(), "k".to_string()],
                        spawn_dim: None,
                    },
                ),
                (
                    "zeta".to_string(),
                    JobConfig {
                        id: "zeta".to_string(),
                        from: vec![
                            JobArg {
                                id: "c".to_string(),
                                over: vec!["k".to_string()],
                            },
                            JobArg {
                                id: "e".to_string(),
                                over: vec!["k".to_string()],
                            },
                        ],
                        to: "f".to_string(),
                        dims: vec!["i".to_string()],
                        spawn_dim: None,
                    },
                ),
            ]),
        };

        let result = trait_service(&all_configs);
        let expected: syn::ItemTrait = parse_quote! {
            #[operon::async_trait::async_trait]
            #[automatically_derived]
            pub trait CookingService: operon::service::OperonService<JobEnum = schema::JobEnum, ResolutionEnum = schema::ResolutionEnum> {
                async fn beta(&self, a: A,) -> Result<Vec<B>, Box<dyn std::error::Error + Send + Sync>>;
                async fn gamma(&self, a: A,) -> Result<Vec<C>, Box<dyn std::error::Error + Send + Sync>>;
                async fn delta(&self, a: A, b: B, c: C,) -> Result<D, Box<dyn std::error::Error + Send + Sync>>;
                async fn epsilon(&self, b_j: Vec<B>, d_j: Vec<D>,) -> Result<E, Box<dyn std::error::Error + Send + Sync>>;
                async fn zeta(&self, c_k: Vec<C>, e_k: Vec<E>,) -> Result<F, Box<dyn std::error::Error + Send + Sync>>;
            }
        };

        assert_eq!(result, expected);
    }
}
