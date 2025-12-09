use syn::parse_quote;

use crate::configs::AllConfig;

fn link_dimension_ids(all_configs: &AllConfig) -> syn::ItemFn {
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
    let link_dimension_ids = link_dimension_ids(all_configs);

    parse_quote! {
        mod misc {
            #link_dimension_ids
        }
    }
}
