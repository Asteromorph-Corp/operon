use quote::quote;
use syn::parse_quote;

use crate::{
    configs::JobConfigMap,
    macros::schema::job::{
        const_job_id::const_job_id, impl_enum_from_job::impl_enum_from_job, impl_job::impl_job,
        impl_job_enum::impl_job_enum, impl_job_sql::impl_job_sql, job_definition::job_definition,
        job_enum::job_enum,
    },
};

pub fn mod_job(jobs: &JobConfigMap) -> syn::ItemMod {
    let const_job_ids = jobs.keys().map(const_job_id);

    let job_enum = job_enum(jobs);
    let impl_job_enum = impl_job_enum();

    let jobs = jobs.values().map(|job| {
        let def = job_definition(job);
        let impl_job = impl_job(job, jobs);
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

            #(#const_job_ids)*

            #job_enum
            #impl_job_enum

            #(#jobs)*
        }
    }
}
