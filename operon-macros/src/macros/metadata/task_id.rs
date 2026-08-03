use syn::parse_quote;

use crate::utils::{task_id_ident, to_lit_str};

/// Generates a constant variable for the given task ID.
///
/// # Example:
/// ```rust,ignore
/// pub const BETA_ID: &str = "beta";
/// ```
pub(super) fn task_id(task_id: &syn::Ident) -> syn::ItemConst {
    let id = task_id_ident(task_id);
    let id_str = to_lit_str(task_id);
    parse_quote! { pub const #id: &str = #id_str; }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use quote::format_ident;

    use super::*;

    #[test]
    fn test_const_task_id() {
        let task = format_ident!("beta");
        let item = task_id(&task);
        let expected: syn::ItemConst = parse_quote! { pub const BETA_ID: &str = "beta"; };
        assert_eq!(item, expected);
    }
}
