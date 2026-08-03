//! This module provides fixtures for a simple pipeline configuration.
//!
//! These fixtures corresponds to a following pipeline:
//! ```rust,ignore
//! alpha = {
//!     A<i> = alpha()
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
    AllConfig, DimensionConfig, DimensionConfigMap, EntityConfig, EntityConfigMap, PoolSizeSpec,
    TaskArg, TaskConfig, TaskConfigMap,
};

pub fn task_alpha() -> TaskConfig {
    TaskConfig {
        id: format_ident!("alpha"),
        from: vec![],
        to: format_ident!("A"),
        dims: vec![],
        spawn_dim: Some(format_ident!("i")),
        pool_size: PoolSizeSpec::Literal(1),
        priority: vec![],
    }
}

pub fn task_beta() -> TaskConfig {
    TaskConfig {
        id: format_ident!("beta"),
        from: vec![TaskArg {
            id: format_ident!("A"),
            over: vec![],
        }],
        to: format_ident!("B"),
        dims: vec![format_ident!("i")],
        spawn_dim: Some(format_ident!("j")),
        pool_size: PoolSizeSpec::Literal(8),
        priority: vec![],
    }
}

pub fn task_gamma() -> TaskConfig {
    TaskConfig {
        id: format_ident!("gamma"),
        from: vec![TaskArg {
            id: format_ident!("A"),
            over: vec![],
        }],
        to: format_ident!("C"),
        dims: vec![format_ident!("i")],
        spawn_dim: Some(format_ident!("k")),
        pool_size: PoolSizeSpec::Literal(8),
        priority: vec![],
    }
}

pub fn task_delta() -> TaskConfig {
    TaskConfig {
        id: format_ident!("delta"),
        from: vec![
            TaskArg {
                id: format_ident!("A"),
                over: vec![],
            },
            TaskArg {
                id: format_ident!("B"),
                over: vec![],
            },
            TaskArg {
                id: format_ident!("C"),
                over: vec![],
            },
        ],
        to: format_ident!("D"),
        dims: vec![format_ident!("i"), format_ident!("j"), format_ident!("k")],
        spawn_dim: None,
        pool_size: PoolSizeSpec::Literal(4),
        priority: vec![],
    }
}

pub fn task_epsilon() -> TaskConfig {
    TaskConfig {
        id: format_ident!("epsilon"),
        from: vec![
            TaskArg {
                id: format_ident!("B"),
                over: vec![format_ident!("j")],
            },
            TaskArg {
                id: format_ident!("D"),
                over: vec![format_ident!("j")],
            },
        ],
        to: format_ident!("E"),
        dims: vec![format_ident!("i"), format_ident!("k")],
        spawn_dim: None,
        pool_size: PoolSizeSpec::Literal(4),
        priority: vec![],
    }
}

pub fn task_zeta() -> TaskConfig {
    TaskConfig {
        id: format_ident!("zeta"),
        from: vec![
            TaskArg {
                id: format_ident!("C"),
                over: vec![format_ident!("k")],
            },
            TaskArg {
                id: format_ident!("E"),
                over: vec![format_ident!("k")],
            },
        ],
        to: format_ident!("F"),
        dims: vec![format_ident!("i")],
        spawn_dim: None,
        pool_size: PoolSizeSpec::Literal(1),
        priority: vec![],
    }
}

#[fixture]
pub fn all_tasks() -> TaskConfigMap {
    TaskConfigMap::from_iter([
        (format_ident!("alpha"), task_alpha()),
        (format_ident!("beta"), task_beta()),
        (format_ident!("gamma"), task_gamma()),
        (format_ident!("delta"), task_delta()),
        (format_ident!("epsilon"), task_epsilon()),
        (format_ident!("zeta"), task_zeta()),
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
        depends_on: vec![format_ident!("i")],
    }
}

#[fixture]
pub fn all_dimensions() -> DimensionConfigMap {
    DimensionConfigMap::from_iter([
        (format_ident!("i"), dimension_i()),
        (format_ident!("j"), dimension_j()),
        (format_ident!("k"), dimension_k()),
    ])
}

pub fn entity_a() -> EntityConfig {
    EntityConfig {
        id: format_ident!("A"),
        dims: vec![format_ident!("i")],
    }
}

pub fn entity_b() -> EntityConfig {
    EntityConfig {
        id: format_ident!("B"),
        dims: vec![format_ident!("i"), format_ident!("j")],
    }
}

pub fn entity_c() -> EntityConfig {
    EntityConfig {
        id: format_ident!("C"),
        dims: vec![format_ident!("i"), format_ident!("k")],
    }
}

pub fn entity_d() -> EntityConfig {
    EntityConfig {
        id: format_ident!("D"),
        dims: vec![format_ident!("i"), format_ident!("j"), format_ident!("k")],
    }
}

pub fn entity_e() -> EntityConfig {
    EntityConfig {
        id: format_ident!("E"),
        dims: vec![format_ident!("i"), format_ident!("k")],
    }
}

pub fn entity_f() -> EntityConfig {
    EntityConfig {
        id: format_ident!("F"),
        dims: vec![format_ident!("i")],
    }
}

#[fixture]
pub fn all_entities() -> EntityConfigMap {
    EntityConfigMap::from_iter([
        (format_ident!("A"), entity_a()),
        (format_ident!("B"), entity_b()),
        (format_ident!("C"), entity_c()),
        (format_ident!("D"), entity_d()),
        (format_ident!("E"), entity_e()),
        (format_ident!("F"), entity_f()),
    ])
}

#[fixture]
pub fn service_id() -> syn::Ident {
    format_ident!("cooking")
}

#[fixture]
pub fn simple_pipeline() -> AllConfig {
    AllConfig {
        service_id: service_id(),
        dimensions: all_dimensions(),
        entities: all_entities(),
        tasks: all_tasks(),
    }
}
