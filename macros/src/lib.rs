// extern crate proc_macro;

mod macros;

mod configs;

mod utils;
use indexmap::IndexMap;
use proc_macro::TokenStream;
use quote::{format_ident, quote};

use crate::{
    configs::{AllConfig, DimensionConfig, EntityConfig, JobArg, JobConfig, JobConfigMap},
    macros::operon,
};

// use utils::{config_types::*, parse::*};

// #[proc_macro]
// pub fn include_operon(input: TokenStream) -> TokenStream {
//     let maybe_config = get_config(input);
//     let config = match maybe_config {
//         Ok(config) => config,
//         Err(e) => return e.to_compile_error().into(),
//     };
//     let operon = macros::write_operon_module(&config);
//     let operon_internal = macros::write_operon_internal_module(&config);
//     quote! {
//         #operon
//         #operon_internal
//     }
//     .into()
// }

#[proc_macro]
pub fn sample_operon(_input: TokenStream) -> TokenStream {
    let all_configs = AllConfig {
        service_id: "Cooking".to_string(),
        primary_dimension: "i".to_string(),
        primary_entity: "a".to_string(),
        dimensions: IndexMap::from_iter([
            (
                "i".to_string(),
                DimensionConfig {
                    id: "i".to_string(),
                    depends_on: vec![],
                },
            ),
            (
                "j".to_string(),
                DimensionConfig {
                    id: "j".to_string(),
                    depends_on: vec!["i".to_string()],
                },
            ),
            (
                "k".to_string(),
                DimensionConfig {
                    id: "k".to_string(),
                    depends_on: vec!["i".to_string()],
                },
            ),
        ]),
        entities: IndexMap::from_iter([
            (
                "a".to_string(),
                EntityConfig {
                    id: "a".to_string(),
                    dims: vec!["i".to_string()],
                    generic: format_ident!("A_"),
                },
            ),
            (
                "b".to_string(),
                EntityConfig {
                    id: "b".to_string(),
                    dims: vec!["i".to_string(), "j".to_string()],
                    generic: format_ident!("B_"),
                },
            ),
            (
                "c".to_string(),
                EntityConfig {
                    id: "c".to_string(),
                    dims: vec!["i".to_string(), "k".to_string()],
                    generic: format_ident!("C_"),
                },
            ),
            (
                "d".to_string(),
                EntityConfig {
                    id: "d".to_string(),
                    dims: vec!["i".to_string(), "j".to_string(), "k".to_string()],
                    generic: format_ident!("D_"),
                },
            ),
            (
                "e".to_string(),
                EntityConfig {
                    id: "e".to_string(),
                    dims: vec!["i".to_string(), "k".to_string()],
                    generic: format_ident!("E_"),
                },
            ),
            (
                "f".to_string(),
                EntityConfig {
                    id: "f".to_string(),
                    dims: vec!["i".to_string()],
                    generic: format_ident!("F_"),
                },
            ),
        ]),
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
                    pool_size: 8,
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
                    pool_size: 8,
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
                    pool_size: 4,
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
                    pool_size: 4,
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
                    pool_size: 1,
                },
            ),
        ]),
    };

    let operon = operon(&all_configs);

    quote! {
        #operon
    }
    .into()
}
