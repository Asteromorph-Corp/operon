use syn::parse_quote;

use crate::utils::{job_id_ident, to_lit_str};

/// Generates a constant variable for the given job ID.
///
/// # Example:
/// ```rust,ignore
/// pub const BETA_ID: &str = "beta";
/// ```
pub(super) fn job_id(job_id: &syn::Ident) -> syn::ItemConst {
    let id = job_id_ident(job_id);
    let id_str = to_lit_str(job_id);
    parse_quote! { pub const #id: &str = #id_str; }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use quote::format_ident;

    use super::*;

    #[test]
    fn test_const_job_id() {
        let job = format_ident!("beta");
        let item = job_id(&job);
        let expected: syn::ItemConst = parse_quote! { pub const BETA_ID: &str = "beta"; };
        assert_eq!(item, expected);
    }
}
