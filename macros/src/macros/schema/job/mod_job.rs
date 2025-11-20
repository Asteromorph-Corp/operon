use syn::parse_quote;

use crate::configs::JobConfigMap;
use crate::macros::schema::job::impl_job_enum::impl_job_enum;
use crate::macros::schema::job::job_enum::job_enum;

/// Generates the `mod job` module with all job-related items.
pub fn mod_job(jobs: &JobConfigMap) -> syn::ItemMod {
    let job_enum = job_enum(jobs);
    let impl_job_enum = impl_job_enum();

    parse_quote! {
        mod job {
            use super::*;

            #job_enum
            #impl_job_enum
        }
    }
}
