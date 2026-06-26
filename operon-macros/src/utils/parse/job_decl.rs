use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::token::Paren;
use syn::{Ident, LitInt, Token};

use super::entity_decl::EntityDecl;

/// Parsed contents of `#[operon(...)]` on a job declaration.
#[derive(Debug, Default)]
pub(super) struct OperonJobAttrs {
    pub(super) priority: Vec<(Ident, bool)>,
}

#[derive(Debug)]
pub(super) struct JobDecl {
    pub(super) spawned_entity: EntityDecl,
    pub(super) _eq_token: Token![=],
    pub(super) id: Ident,
    pub(super) _paren_args_token: Paren,
    pub(super) args: Vec<EntityDecl>,
    pub(super) _for_token: Option<Token![for]>,
    pub(super) _paren_pool_token: Option<Paren>,
    pub(super) pool: Option<LitInt>,
    pub(super) dims: Vec<Ident>,
    pub(super) _semi_token: Token![;],
    pub(super) operon_attrs: OperonJobAttrs,
    pub(super) _span: proc_macro2::Span,
}
impl JobDecl {
    fn validate(&self) -> syn::Result<()> {
        if self.spawned_entity.dims.len() > 1 {
            return Err(syn::Error::new(
                self.spawned_entity._span,
                format!(
                    "Job '{}' can only spawn an entity with one dimension, found {} dimensions",
                    self.id,
                    self.spawned_entity.dims.len()
                ),
            ));
        }
        if self._for_token.is_none() && self.pool.is_some() {
            return Err(syn::Error::new(
                self._span,
                format!("Job '{}' has a pool specified but no 'for' clause", self.id),
            ));
        }
        if self._for_token.is_none() && !self.dims.is_empty() {
            return Err(syn::Error::new(
                self._span,
                format!(
                    "Job '{}' has dimensions specified but no 'for' clause",
                    self.id
                ),
            ));
        }
        if let Some(for_token) = self._for_token
            && self.dims.is_empty()
        {
            return Err(syn::Error::new(
                for_token.span(),
                format!(
                    "Job '{}' has a 'for' clause but no dimensions are specified",
                    self.id
                ),
            ));
        }
        if let Some(ref pool) = self.pool {
            let Ok(pool_val) = pool.base10_parse::<usize>() else {
                return Err(syn::Error::new(
                    pool.span(),
                    format!("Job '{}' has an invalid pool value: '{}'", self.id, pool),
                ));
            };
            if pool_val == 0 {
                return Err(syn::Error::new(
                    pool.span(),
                    format!(
                        "Job '{}' has a pool value of 0, which is not allowed",
                        self.id
                    ),
                ));
            }
        }
        for (dim, _) in &self.operon_attrs.priority {
            if !self.dims.iter().any(|d| d == dim) {
                return Err(syn::Error::new(
                    dim.span(),
                    format!(
                        "Priority dimension '{}' is not in the dimension set of job '{}'",
                        dim, self.id
                    ),
                ));
            }
        }
        for (i, (dim, _)) in self.operon_attrs.priority.iter().enumerate() {
            if self.operon_attrs.priority[..i].iter().any(|(d, _)| d == dim) {
                return Err(syn::Error::new(
                    dim.span(),
                    format!(
                        "Priority dimension '{}' appears more than once in job '{}'",
                        dim, self.id
                    ),
                ));
            }
        }
        Ok(())
    }
}
impl Parse for JobDecl {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let start = input.span();

        // Parse optional #[operon(...)] attribute before the job line.
        let attrs = input.call(syn::Attribute::parse_outer)?;
        let operon_attrs = parse_operon_attrs(&attrs)?;

        let spawned_entity: EntityDecl = input.parse()?;
        let eq_token: Token![=] = input.parse()?;
        let id: Ident = input.parse()?;
        let args_content;
        let paren_args_token: Paren = syn::parenthesized!(args_content in input);
        let args: Vec<EntityDecl> = {
            let mut args = Vec::new();
            while !args_content.is_empty() {
                args.push(args_content.parse()?);
                if !args_content.is_empty() {
                    args_content.parse::<Token![,]>()?;
                }
            }
            args
        };
        let for_token = if input.peek(Token![for]) {
            Some(input.parse()?)
        } else {
            None
        };
        let (paren_pool_token, pool) = if input.peek(Paren) {
            let content;
            let paren_pool_token: Paren = syn::parenthesized!(content in input);
            let pool: LitInt = content.parse()?;
            (Some(paren_pool_token), Some(pool))
        } else {
            (None, None)
        };
        let dims = {
            let mut dims = Vec::new();
            while !input.is_empty() && !input.peek(Token![;]) {
                dims.push(input.parse()?);
                if !input.is_empty() && !input.peek(Token![;]) {
                    input.parse::<Token![,]>()?;
                }
            }
            dims
        };
        let semi_token: Token![;] = input.parse()?;
        let end = input.span();
        let _span = start.join(end).unwrap_or(start);
        let job_decl = JobDecl {
            spawned_entity,
            _eq_token: eq_token,
            id,
            _paren_args_token: paren_args_token,
            args,
            _for_token: for_token,
            _paren_pool_token: paren_pool_token,
            pool,
            dims,
            _semi_token: semi_token,
            operon_attrs,
            _span,
        };
        job_decl.validate()?;
        Ok(job_decl)
    }
}

/// Parses `#[operon(...)]` attributes on a job declaration into [`OperonJobAttrs`].
/// Rejects non-`operon` attributes and unknown keys inside `#[operon(...)]`.
fn parse_operon_attrs(attrs: &[syn::Attribute]) -> syn::Result<OperonJobAttrs> {
    let mut result = OperonJobAttrs::default();
    for attr in attrs {
        if !attr.path().is_ident("operon") {
            return Err(syn::Error::new_spanned(
                attr,
                "Unknown attribute on job declaration; only `#[operon(...)]` is supported",
            ));
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("ord") {
                let value = meta.value()?;
                let content;
                syn::parenthesized!(content in value);
                while !content.is_empty() {
                    let descending = content.peek(Token![-]);
                    if descending {
                        content.parse::<Token![-]>()?;
                    }
                    let dim: Ident = content.parse()?;
                    result.priority.push((dim, descending));
                    if !content.is_empty() {
                        content.parse::<Token![,]>()?;
                    }
                }
                Ok(())
            } else {
                Err(meta.error("Unknown key in `#[operon(...)]`; accepted keys are: {`ord`}"))
            }
        })?;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use syn::parse_str;

    use super::*;

    #[test]
    fn test_job_decl_simple() {
        let input = "Entity = JobName();";
        let parsed: JobDecl = parse_str(input).expect("Failed to parse");
        assert_eq!(parsed.spawned_entity.id.to_string(), "Entity");
        assert_eq!(parsed.id.to_string(), "JobName");
        assert!(parsed.args.is_empty());
        assert!(parsed._for_token.is_none());
        assert!(parsed.pool.is_none());
        assert!(parsed.dims.is_empty());
        assert!(parsed.operon_attrs.priority.is_empty());
    }
    #[test]
    fn test_job_decl_all() {
        let beta = "B<j> = beta(A) for(8) i;";
        let gamma = "C<k> = gamma(A) for(8) i;";
        let delta = "D = delta(A, B, C) for(4) i, j, k;";
        let epsilon = "E = epsilon(B<j>, D<j>) for(4) i, k;";
        let zeta = "F = zeta(C<k>, E<k>) for i;";
        let parsed_beta: JobDecl = parse_str(beta).expect("Failed to parse beta");
        let parsed_gamma: JobDecl = parse_str(gamma).expect("Failed to parse gamma");
        let parsed_delta: JobDecl = parse_str(delta).expect("Failed to parse delta");
        let parsed_epsilon: JobDecl = parse_str(epsilon).expect("Failed to parse epsilon");
        let parsed_zeta: JobDecl = parse_str(zeta).expect("Failed to parse zeta");
        assert_eq!(parsed_beta.id.to_string(), "beta");
        assert_eq!(parsed_beta.spawned_entity.id.to_string(), "B");
        assert_eq!(parsed_beta.spawned_entity.dims.len(), 1);
        assert_eq!(
            parsed_gamma.pool.unwrap().base10_parse::<usize>().unwrap(),
            8
        );
        assert_eq!(parsed_gamma.args.len(), 1);
        assert_eq!(parsed_gamma.args[0].id.to_string(), "A");
        assert_eq!(parsed_delta.dims.len(), 3);
        assert_eq!(parsed_delta.dims[0].to_string(), "i");
        assert_eq!(parsed_epsilon.args.len(), 2);
        assert_eq!(parsed_epsilon.args[0].dims.len(), 1);
        assert_eq!(parsed_epsilon.args[0].dims[0].to_string(), "j");
        assert!(parsed_zeta.pool.is_none());
    }

    #[test]
    fn test_job_decl_priority_valid_dims() {
        let input = "#[operon(ord=(-k, i))] E = epsilon(B<j>, D<j>) for(4) i, k;";
        let parsed: JobDecl = parse_str(input).expect("Failed to parse");
        assert_eq!(parsed.operon_attrs.priority.len(), 2);
        assert_eq!(parsed.operon_attrs.priority[0].0.to_string(), "k");
        assert!(parsed.operon_attrs.priority[0].1, "k should be descending");
        assert_eq!(parsed.operon_attrs.priority[1].0.to_string(), "i");
        assert!(!parsed.operon_attrs.priority[1].1, "i should be ascending");
    }

    #[test]
    fn test_job_decl_malformed() {
        let malformed_inputs = [
            "Entity = JobName() for i",                          // Missing semicolon
            "Entity = JobName() for(8 i, j;",                    // Missing closing parenthesis
            "Entity = JobName() for;",                           // Redundant for
            "Entity<i, j> = JobName() for(8) i, j;",             // Too many dimensions
            "Entity = JobName for(8) i;",                        // Missing argument parens
            "Entity = JobName() for i, j k;",                    // Missing comma
            "Entity = JobName() for(8) i, j, k; SomeExtraToken", // Extra token after semicolon
            "#[operon(ord=(z))] E = job() for i;",               // Unknown priority dim
            "#[operon(ord=(i, i))] E = job() for i;",            // Duplicate priority dim
            "#[unknown] E = job() for i;",                       // Unknown attribute
            "#[operon(unknown_key)] E = job() for i;",           // Unknown operon key
        ];
        let results = malformed_inputs
            .into_iter()
            .map(parse_str::<JobDecl>)
            .collect::<Vec<_>>();
        assert!(results.iter().all(|result| result.is_err()));
    }
}
