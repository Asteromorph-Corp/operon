use proc_macro_error::{Diagnostic, Level};
use syn::parse_str;

#[derive(Default)]
pub struct OperonAttrs {
    pub crate_path: Option<syn::Path>,
    pub definition_path: Option<syn::Path>,
    pub error_type: Option<syn::Type>,
}

pub fn extract_attr<T: Default>(
    attrs: &[syn::Attribute],
    get_attr: impl Fn(&syn::Attribute) -> Result<Option<T>, Diagnostic>,
) -> Result<T, Diagnostic> {
    attrs
        .iter()
        .find_map(|attr| get_attr(attr).transpose())
        .unwrap_or_else(|| Ok(T::default()))
}

pub fn get_operon_attrs(attr: &syn::Attribute) -> Result<Option<OperonAttrs>, Diagnostic> {
    if !attr.path().is_ident("operon") {
        return Ok(None);
    }

    let mut crate_path = None;
    let mut definition_path = None;
    let mut error_type = None;

    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("crate") {
            let value = meta.value()?;
            let lit: syn::LitStr = value.parse()?;
            let path = parse_str::<syn::Path>(&lit.value())?;
            crate_path = Some(path)
        }
        if meta.path.is_ident("defined_at") {
            let value = meta.value()?;
            let lit: syn::LitStr = value.parse()?;
            let path = parse_str::<syn::Path>(&lit.value())?;
            definition_path = Some(path)
        }
        if meta.path.is_ident("error") {
            let value = meta.value()?;
            let lit: syn::LitStr = value.parse()?;
            let ty = parse_str::<syn::Type>(&lit.value())?;
            error_type = Some(ty)
        }

        Ok(())
    })
    .map_err(|e| {
        Diagnostic::spanned(
            e.span(),
            Level::Error,
            format!("Failed to parse operon attribute: {e}"),
        )
    })?;

    Ok(Some(OperonAttrs {
        crate_path,
        definition_path,
        error_type,
    }))
}
