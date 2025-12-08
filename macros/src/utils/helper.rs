pub fn dim_msg(dim: &syn::Ident) -> String {
    format!("{dim} = {{{dim}}}")
}
