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
    AllConfig, DimensionConfig, DimensionConfigMap, EntityConfig, EntityConfigMap, PoolSizeSpec,
    TaskArg, TaskConfig, TaskConfigMap,
};

pub fn task_alpha() -> TaskConfig {
    TaskConfig {
        id: format_ident!("alpha"),
        from: vec![],
        to: format_ident!("A"),
        dims: vec![],
        spawn_dim: None,
        pool_size: PoolSizeSpec::Literal(1),
        priority: vec![],
    }
}

pub fn task_beta() -> TaskConfig {
    TaskConfig {
        id: format_ident!("beta"),
        from: vec![],
        to: format_ident!("B"),
        dims: vec![],
        spawn_dim: Some(format_ident!("i")),
        pool_size: PoolSizeSpec::Literal(1),
        priority: vec![],
    }
}

pub fn task_gamma() -> TaskConfig {
    TaskConfig {
        id: format_ident!("gamma"),
        from: vec![
            TaskArg {
                id: format_ident!("A"),
                over: vec![],
            },
            TaskArg {
                id: format_ident!("B"),
                over: vec![],
            },
        ],
        to: format_ident!("C"),
        dims: vec![format_ident!("i")],
        spawn_dim: Some(format_ident!("j")),
        pool_size: PoolSizeSpec::Literal(8),
        priority: vec![],
    }
}

pub fn task_delta() -> TaskConfig {
    TaskConfig {
        id: format_ident!("delta"),
        from: vec![TaskArg {
            id: format_ident!("B"),
            over: vec![],
        }],
        to: format_ident!("D"),
        dims: vec![format_ident!("i"), format_ident!("j")],
        spawn_dim: Some(format_ident!("k")),
        pool_size: PoolSizeSpec::Literal(4),
        priority: vec![],
    }
}

pub fn task_epsilon() -> TaskConfig {
    TaskConfig {
        id: format_ident!("epsilon"),
        from: vec![
            TaskArg {
                id: format_ident!("C"),
                over: vec![format_ident!("j")],
            },
            TaskArg {
                id: format_ident!("D"),
                over: vec![format_ident!("j"), format_ident!("k")],
            },
        ],
        to: format_ident!("E"),
        dims: vec![format_ident!("i")],
        spawn_dim: Some(format_ident!("l")),
        pool_size: PoolSizeSpec::Literal(2),
        priority: vec![],
    }
}

pub fn all_tasks() -> TaskConfigMap {
    TaskConfigMap::from_iter([
        (format_ident!("alpha"), task_alpha()),
        (format_ident!("beta"), task_beta()),
        (format_ident!("gamma"), task_gamma()),
        (format_ident!("delta"), task_delta()),
        (format_ident!("epsilon"), task_epsilon()),
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
        id: format_ident!("A"),
        dims: vec![],
    }
}

pub fn entity_b() -> EntityConfig {
    EntityConfig {
        id: format_ident!("B"),
        dims: vec![format_ident!("i")],
    }
}

pub fn entity_c() -> EntityConfig {
    EntityConfig {
        id: format_ident!("C"),
        dims: vec![format_ident!("j")],
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
        dims: vec![format_ident!("l")],
    }
}

pub fn all_entities() -> EntityConfigMap {
    EntityConfigMap::from_iter([
        (format_ident!("A"), entity_a()),
        (format_ident!("B"), entity_b()),
        (format_ident!("C"), entity_c()),
        (format_ident!("D"), entity_d()),
        (format_ident!("E"), entity_e()),
    ])
}

pub fn service_id() -> syn::Ident {
    format_ident!("complicated")
}

#[allow(dead_code)]
pub fn complicated_pipeline() -> AllConfig {
    AllConfig {
        service_id: service_id(),
        dimensions: all_dimensions(),
        entities: all_entities(),
        tasks: all_tasks(),
    }
}
