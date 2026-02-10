use syn::parse_quote;

use crate::configs::AllConfig;
use crate::operon_ident;
use crate::utils::{service_trait_ident, spec_ident, storage_trait_ident};

fn fn_scheduler_handler(all_configs: &AllConfig) -> syn::ItemFn {
    let operon = operon_ident();
    let service_trait = service_trait_ident(&all_configs.service_id);
    let storage_trait = storage_trait_ident(&all_configs.service_id);
    let specs = all_configs.jobs.keys().map(spec_ident);

    parse_quote! {
        pub fn scheduler_handler<Svc: #service_trait, Sto: #storage_trait>() -> #operon::scheduler::SchedulerHandler<Svc, Sto> {
            #operon::scheduler::SchedulerHandler::new(vec![
                #(spec::#specs.into_handler(),)*
            ])
        }
    }
}

fn fn_link_dimension_ids(all_configs: &AllConfig) -> syn::ItemFn {
    let defs = all_configs.jobs.values().filter_map(|job| {
        let spawn_dim = job.spawn_dim.as_ref()?;
        let doc = format!("Dimension spawned by `{}`", job.id);
        let stmt: syn::Field = parse_quote! {
            #[doc = #doc]
            #spawn_dim: Dimension
        };
        Some(stmt)
    });
    let refs = all_configs
        .jobs
        .values()
        .flat_map(|job| job.dims.iter())
        .chain(
            all_configs
                .jobs
                .values()
                .flat_map(|job| job.from.iter().flat_map(|arg| arg.over.iter())),
        );

    parse_quote! {
        fn _link_dimsnion_ids() {
            #[derive(Default)]
            struct Dimension;
            #[derive(Default)]
            struct Dimensions {
                #(#defs,)*
            }
            let x: Dimensions = Dimensions::default();

            #(let _ = x.#refs;)*
        }

    }
}

pub fn misc(all_configs: &AllConfig) -> syn::ItemMod {
    let fn_scheduler_handler = fn_scheduler_handler(all_configs);
    let fn_link_dimension_ids = fn_link_dimension_ids(all_configs);
    let storage_trait = storage_trait_ident(&all_configs.service_id);

    parse_quote! {
        pub mod __misc {
            use super::*;

            #fn_scheduler_handler
            #fn_link_dimension_ids
            pub use super::#storage_trait as StorageTrait;
        }
    }
}
