use heck::ToSnakeCase;
use syn::parse_quote;

use crate::configs::JobId;
use crate::utils::job_id_ident;

/// Generates a constant variable for the given job ID.
///
/// Example:
/// ```rust,ignore
/// const BETA_ID: &str = "beta";
/// ```
pub(super) fn const_job_id(job_id: &JobId) -> syn::ItemConst {
    let id_ident = job_id_ident(job_id);
    let id = job_id.to_snake_case();

    parse_quote! {
        const #id_ident: &str = #id;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_const_job_id() {
        let job = "beta".to_string();

        let item = const_job_id(&job);
        let expected: syn::ItemConst = parse_quote! {
            const BETA_ID: &str = "beta";
        };

        assert_eq!(item, expected);
    }
}
