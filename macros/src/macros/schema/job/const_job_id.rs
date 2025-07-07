use heck::ToSnakeCase;
use syn::parse_quote;

use crate::{configs::JobId, utils::job_id_ident};

pub(super) fn const_job_id(job_id: &JobId) -> syn::ItemConst {
    let id_ident = job_id_ident(job_id);
    let id = job_id.to_snake_case();

    parse_quote! {
        const #id_ident: &str = #id;
    }
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;

    use super::*;

    #[test]
    fn test_const_job_id() {
        let job = "beta".to_string();

        let item = const_job_id(&job);
        let expected: syn::ItemConst = parse_quote! {
            const BETA_ID: &str = "beta";
        };

        assert_eq!(
            item.to_token_stream().to_string(),
            expected.to_token_stream().to_string()
        );
    }
}
