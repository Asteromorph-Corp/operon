use syn::parse::{Parse, ParseStream};
use syn::{Ident, Token};

#[derive(Debug)]
pub(super) struct EntityDecl {
    pub(super) id: Ident,
    pub(super) _lt_token: Option<Token![<]>,
    pub(super) dims: Vec<Ident>,
    pub(super) _gt_token: Option<Token![>]>,
    pub(super) _span: proc_macro2::Span,
}
impl Parse for EntityDecl {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let start = input.span();
        let id: Ident = input.parse()?;
        let lt_token = if input.peek(Token![<]) {
            Some(input.parse()?)
        } else {
            None
        };
        let dims = if lt_token.is_some() {
            let mut dims = Vec::new();
            while !input.is_empty() && !input.peek(Token![>]) {
                dims.push(input.parse()?);
                if !input.is_empty() && !input.peek(Token![>]) {
                    let _ = input.parse::<Token![,]>()?;
                }
            }
            dims
        } else {
            Vec::new()
        };
        let gt_token = if input.peek(Token![>]) {
            Some(input.parse()?)
        } else {
            None
        };
        let end = input.span();
        let _span = start.join(end).unwrap_or(start);
        match (lt_token, gt_token) {
            (Some(_), None) => {
                return Err(syn::Error::new(
                    input.span(),
                    "Expected '>' to close the entity declaration",
                ));
            }
            (None, Some(_)) => {
                return Err(syn::Error::new(
                    input.span(),
                    "Unexpected '>' without a preceding '<'",
                ));
            }
            _ => {}
        }
        Ok(EntityDecl {
            id,
            _lt_token: lt_token,
            dims,
            _gt_token: gt_token,
            _span,
        })
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_str;

    use super::*;

    #[test]
    fn test_entity_decl_simple() {
        let input = "Entity";
        let parsed: EntityDecl = parse_str(input).expect("Failed to parse");

        assert_eq!(parsed.id.to_string(), "Entity");
        assert!(parsed._lt_token.is_none());
        assert!(parsed.dims.is_empty());
        assert!(parsed._gt_token.is_none());
    }

    #[test]
    fn test_entity_decl_with_dims() {
        let input = "Entity<A, B, C>";
        let parsed: EntityDecl = parse_str(input).expect("Failed to parse");

        assert_eq!(parsed.id.to_string(), "Entity");
        assert!(parsed._lt_token.is_some());
        assert_eq!(parsed.dims.len(), 3);
        assert_eq!(
            parsed
                .dims
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>(),
            ["A", "B", "C"]
        );
        assert!(parsed._gt_token.is_some());
    }

    #[test]
    fn test_entity_decl_with_one_dim() {
        let input = "Entity<X,>";
        let parsed: EntityDecl = parse_str(input).expect("Failed to parse");

        assert_eq!(parsed.id.to_string(), "Entity");
        assert!(parsed._lt_token.is_some());
        assert_eq!(parsed.dims.len(), 1);
        assert_eq!(parsed.dims[0].to_string(), "X");
        assert!(parsed._gt_token.is_some());
    }

    #[test]
    fn test_entity_decl_missing_gt_should_fail() {
        let input = "Entity<A, B";
        let result: syn::Result<EntityDecl> = parse_str(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_entity_decl_unexpected_gt_should_fail() {
        let input = "Entity A, B>";
        let result: syn::Result<EntityDecl> = parse_str(input);
        assert!(result.is_err());
    }
}
