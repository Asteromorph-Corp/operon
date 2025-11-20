use quote::quote;
use syn::parse_quote;

use crate::configs::JobConfigMap;
use crate::macros::schema::job::impl_enum_from_job::impl_enum_from_job;
use crate::macros::schema::job::impl_job::impl_job;
use crate::macros::schema::job::impl_job_enum::impl_job_enum;
use crate::macros::schema::job::impl_job_sql::impl_job_sql;
use crate::macros::schema::job::job_definition::job_definition;
use crate::macros::schema::job::job_enum::job_enum;

/// Generates the `mod job` module with all job-related items.
pub fn mod_job(jobs: &JobConfigMap) -> syn::ItemMod {
    let job_enum = job_enum(jobs);
    let impl_job_enum = impl_job_enum();

    let jobs = jobs.values().map(|job| {
        let def = job_definition(job);
        let impl_job = impl_job(job);
        let impl_job_sql = impl_job_sql(job);
        let impl_enum_from_job = impl_enum_from_job(job);

        quote! {
            #def
            #impl_job
            #impl_job_sql
            #impl_enum_from_job
        }
    });

    parse_quote! {
        mod job {
            use super::*;

            #job_enum
            #impl_job_enum

            #(#jobs)*
        }
    }
}
