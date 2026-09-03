use syn::parse_quote;

use crate::configs::TaskConfigMap;
use crate::operon_ident;
use crate::utils::{job_enum_ident, to_pascal_case};

/// Generates an enum representing the job of any task in the task configuration map.
///
/// # Example
/// ```rust,ignore
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/schema/job/job_enum_definition.rs") )]
/// ```
pub fn job_enum_definition(tasks: &TaskConfigMap) -> syn::ItemEnum {
    let operon = operon_ident();
    let job_enum_ident = job_enum_ident();
    let variants = tasks.values().map(|task| -> syn::Variant {
        let variant_ident = to_pascal_case(&task.id);
        let n = task.dims.len();
        parse_quote! {
            #variant_ident(#operon::__private::Job<#n>)
        }
    });
    let doc = "An enum representing any job.";

    parse_quote! {
        #[doc = #doc]
        #[derive(Debug, Clone)]
        pub enum #job_enum_ident {
            #(#variants,)*
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::test_utils::assert_item_eq;
    use crate::test_utils::simple_pipeline::all_tasks;

    #[rstest]
    fn test_job_enum_definition(all_tasks: TaskConfigMap) {
        let result = job_enum_definition(&all_tasks);
        assert_item_eq(&result, "schema/job/job_enum_definition.rs");
    }
}
