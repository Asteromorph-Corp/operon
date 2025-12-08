//! This module provides fixtures for a simple pipeline configuration.
//!
//! These fixtures corresponds to a following pipeline:
//! ```rust,ignore
//! alpha = {
//!     A = alpha();
//!     B<i> = beta();
//!     C<j> = gamma(A, B) for(8) i;
//!     D<k> = delta(C) for(4) i, j;
//!     E<l> = epsilon(C<j>, D<j, k>) for(2) i;
//! }
//! ```

use quote::format_ident;

use crate::configs::{
    AllConfig, DimensionConfig, DimensionConfigMap, EntityConfig, EntityConfigMap, JobArg,
    JobConfig, JobConfigMap,
};

pub fn job_alpha() -> JobConfig {
    JobConfig {
        id: "alpha".to_string(),
        from: vec![],
        to: "a".to_string(),
        dims: vec![],
        spawn_dim: None,
        pool_size: 1,
    }
}

pub fn job_beta() -> JobConfig {
    JobConfig {
        id: "beta".to_string(),
        from: vec![],
        to: "b".to_string(),
        dims: vec![],
        spawn_dim: Some(format_ident!("i")),
        pool_size: 1,
    }
}

pub fn job_gamma() -> JobConfig {
    JobConfig {
        id: "gamma".to_string(),
        from: vec![
            JobArg {
                id: "a".to_string(),
                over: vec![],
            },
            JobArg {
                id: "b".to_string(),
                over: vec![],
            },
        ],
        to: "c".to_string(),
        dims: vec![format_ident!("i")],
        spawn_dim: Some(format_ident!("j")),
        pool_size: 8,
    }
}

pub fn job_delta() -> JobConfig {
    JobConfig {
        id: "delta".to_string(),
        from: vec![JobArg {
            id: "b".to_string(),
            over: vec![],
        }],
        to: "d".to_string(),
        dims: vec![format_ident!("i"), format_ident!("j")],
        spawn_dim: Some(format_ident!("k")),
        pool_size: 4,
    }
}

pub fn job_epsilon() -> JobConfig {
    JobConfig {
        id: "epsilon".to_string(),
        from: vec![
            JobArg {
                id: "c".to_string(),
                over: vec![format_ident!("j")],
            },
            JobArg {
                id: "d".to_string(),
                over: vec![format_ident!("j"), format_ident!("k")],
            },
        ],
        to: "e".to_string(),
        dims: vec![format_ident!("i")],
        spawn_dim: Some(format_ident!("l")),
        pool_size: 2,
    }
}

pub fn all_jobs() -> JobConfigMap {
    JobConfigMap::from_iter([
        ("alpha".to_string(), job_alpha()),
        ("beta".to_string(), job_beta()),
        ("gamma".to_string(), job_gamma()),
        ("delta".to_string(), job_delta()),
        ("epsilon".to_string(), job_epsilon()),
    ])
}

pub fn dimension_i() -> DimensionConfig {
    DimensionConfig {
        id: format_ident!("i"),
        depends_on: vec![],
    }
}

pub fn dimension_j() -> DimensionConfig {
    DimensionConfig {
        id: format_ident!("j"),
        depends_on: vec![format_ident!("i")],
    }
}

pub fn dimension_k() -> DimensionConfig {
    DimensionConfig {
        id: format_ident!("k"),
        depends_on: vec![format_ident!("i"), format_ident!("j")],
    }
}

pub fn dimension_l() -> DimensionConfig {
    DimensionConfig {
        id: format_ident!("l"),
        depends_on: vec![],
    }
}

pub fn all_dimensions() -> DimensionConfigMap {
    DimensionConfigMap::from_iter([
        (format_ident!("i"), dimension_i()),
        (format_ident!("j"), dimension_j()),
        (format_ident!("k"), dimension_k()),
        (format_ident!("l"), dimension_l()),
    ])
}

pub fn entity_a() -> EntityConfig {
    EntityConfig {
        id: "a".to_string(),
        dims: vec![],
    }
}

pub fn entity_b() -> EntityConfig {
    EntityConfig {
        id: "b".to_string(),
        dims: vec![format_ident!("i")],
    }
}

pub fn entity_c() -> EntityConfig {
    EntityConfig {
        id: "c".to_string(),
        dims: vec![format_ident!("j")],
    }
}

pub fn entity_d() -> EntityConfig {
    EntityConfig {
        id: "d".to_string(),
        dims: vec![format_ident!("i"), format_ident!("j"), format_ident!("k")],
    }
}

pub fn entity_e() -> EntityConfig {
    EntityConfig {
        id: "e".to_string(),
        dims: vec![format_ident!("l")],
    }
}

pub fn all_entities() -> EntityConfigMap {
    EntityConfigMap::from_iter([
        ("a".to_string(), entity_a()),
        ("b".to_string(), entity_b()),
        ("c".to_string(), entity_c()),
        ("d".to_string(), entity_d()),
        ("e".to_string(), entity_e()),
    ])
}

pub fn service_id() -> &'static str {
    "Complicated"
}

#[allow(dead_code)]
pub fn complicated_pipeline() -> AllConfig {
    AllConfig {
        service_id: service_id().to_string(),
        dimensions: all_dimensions(),
        entities: all_entities(),
        jobs: all_jobs(),
    }
}
