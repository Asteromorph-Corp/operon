//! This module provides fixtures for a simple pipeline configuration.
//!
//! These fixtures corresponds to a following pipeline:
//! ```rust,ignore
//! alpha = |A<i>| {
//!     B<j> = beta(A) for(8) i;
//!     C<k> = gamma(A) for(8) i, j;
//!     D = delta(A, B, C) for(4) i, j, k;
//!     E = epsilon(B<j>, D<j>) for(4) i, k;
//!     F = zeta(C<k>, E<k>) for(1) i;
//! }
//! ```

use quote::format_ident;
use rstest::fixture;

use crate::configs::{
    AllConfig, DimensionConfig, DimensionConfigMap, DimensionId, EntityConfig, EntityConfigMap,
    JobArg, JobConfig, JobConfigMap,
};

pub fn job_beta() -> JobConfig {
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
    }
}

pub fn job_gamma() -> JobConfig {
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
    }
}

pub fn job_delta() -> JobConfig {
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
    }
}

pub fn job_epsilon() -> JobConfig {
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
    }
}

pub fn job_zeta() -> JobConfig {
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
    }
}

#[fixture]
pub fn all_jobs() -> JobConfigMap {
    JobConfigMap::from_iter([
        ("beta".to_string(), job_beta()),
        ("gamma".to_string(), job_gamma()),
        ("delta".to_string(), job_delta()),
        ("epsilon".to_string(), job_epsilon()),
        ("zeta".to_string(), job_zeta()),
    ])
}

pub fn dimension_i() -> DimensionConfig {
    DimensionConfig {
        id: "i".to_string(),
        depends_on: vec![],
    }
}

pub fn dimension_j() -> DimensionConfig {
    DimensionConfig {
        id: "j".to_string(),
        depends_on: vec!["i".to_string()],
    }
}

pub fn dimension_k() -> DimensionConfig {
    DimensionConfig {
        id: "k".to_string(),
        depends_on: vec!["i".to_string()],
    }
}

#[fixture]
pub fn all_dimensions() -> DimensionConfigMap {
    DimensionConfigMap::from_iter([
        ("i".to_string(), dimension_i()),
        ("j".to_string(), dimension_j()),
        ("k".to_string(), dimension_k()),
    ])
}

pub fn entity_a() -> EntityConfig {
    EntityConfig {
        id: "a".to_string(),
        dims: vec!["i".to_string()],
        generic: format_ident!("A_"),
    }
}

pub fn entity_b() -> EntityConfig {
    EntityConfig {
        id: "b".to_string(),
        dims: vec!["i".to_string(), "j".to_string()],
        generic: format_ident!("B_"),
    }
}

pub fn entity_c() -> EntityConfig {
    EntityConfig {
        id: "c".to_string(),
        dims: vec!["i".to_string(), "k".to_string()],
        generic: format_ident!("C_"),
    }
}

pub fn entity_d() -> EntityConfig {
    EntityConfig {
        id: "d".to_string(),
        dims: vec!["i".to_string(), "j".to_string(), "k".to_string()],
        generic: format_ident!("D_"),
    }
}

pub fn entity_e() -> EntityConfig {
    EntityConfig {
        id: "e".to_string(),
        dims: vec!["i".to_string(), "k".to_string()],
        generic: format_ident!("E_"),
    }
}

pub fn entity_f() -> EntityConfig {
    EntityConfig {
        id: "f".to_string(),
        dims: vec!["i".to_string()],
        generic: format_ident!("F_"),
    }
}

#[fixture]
pub fn all_entities() -> EntityConfigMap {
    EntityConfigMap::from_iter([
        ("a".to_string(), entity_a()),
        ("b".to_string(), entity_b()),
        ("c".to_string(), entity_c()),
        ("d".to_string(), entity_d()),
        ("e".to_string(), entity_e()),
        ("f".to_string(), entity_f()),
    ])
}

#[fixture]
pub fn service_id() -> &'static str {
    "Cooking"
}

#[fixture]
pub fn simple_pipeline() -> AllConfig {
    AllConfig {
        service_id: service_id().to_string(),
        dimensions: all_dimensions(),
        entities: all_entities(),
        jobs: all_jobs(),
    }
}
