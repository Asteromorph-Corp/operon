use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::token::Paren;
use syn::{Ident, LitInt, Token};

use super::entity_decl::EntityDecl;
use crate::configs::{Direction, PoolSizeSpec};

/// Parsed contents of `#[operon(...)]` on a job declaration.
#[derive(Debug, Default)]
pub(super) struct OperonJobAttrs {
    pub(super) priority: Option<Vec<(Ident, Direction)>>,
    pub(super) concurrency: Option<PoolSizeSpec>,
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
                    "Task '{}' can only spawn an entity with one dimension, found {} dimensions",
                    self.id,
                    self.spawned_entity.dims.len()
                ),
            ));
        }
        if self._for_token.is_none() && self.pool.is_some() {
            return Err(syn::Error::new(
                self._span,
                format!(
                    "Task '{}' has a pool specified but no 'for' clause",
                    self.id
                ),
            ));
        }
        if self._for_token.is_none() && !self.dims.is_empty() {
            return Err(syn::Error::new(
                self._span,
                format!(
                    "Task '{}' has dimensions specified but no 'for' clause",
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
                    "Task '{}' has a 'for' clause but no dimensions are specified",
                    self.id
                ),
            ));
        }
        if let Some(ref pool) = self.pool {
            let Ok(pool_val) = pool.base10_parse::<usize>() else {
                return Err(syn::Error::new(
                    pool.span(),
                    format!("Task '{}' has an invalid pool value: '{}'", self.id, pool),
                ));
            };
            if pool_val == 0 {
                return Err(syn::Error::new(
                    pool.span(),
                    format!(
                        "Task '{}' has a pool value of 0, which is not allowed",
                        self.id
                    ),
                ));
            }
        }
        if let Some(priority) = &self.operon_attrs.priority {
            for (dim, _) in priority {
                if !self.dims.iter().any(|d| d == dim) {
                    return Err(syn::Error::new(
                        dim.span(),
                        format!(
                            "Priority dimension '{}' is not in the dimension set of task '{}'",
                            dim, self.id
                        ),
                    ));
                }
            }
        }
        if let Some(priority) = &self.operon_attrs.priority {
            for (i, (dim, _)) in priority.iter().enumerate() {
                if priority[..i].iter().any(|(d, _)| d == dim) {
                    return Err(syn::Error::new(
                        dim.span(),
                        format!(
                            "Priority dimension '{}' appears more than once in task '{}'",
                            dim, self.id
                        ),
                    ));
                }
            }
        }
        if self.pool.is_some() && self.operon_attrs.concurrency.is_some() {
            return Err(syn::Error::new(
                self._span,
                format!(
                    "Task '{}' specifies concurrency via both 'for(N)' and '#[operon(...)]'; use only one",
                    self.id
                ),
            ));
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
                "Unknown attribute on task declaration; only `#[operon(...)]` is supported",
            ));
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("ord") {
                if result.priority.is_some() {
                    return Err(meta.error("Multiple `ord` keys found in `#[operon(...)]`"));
                }
                let mut priority = Vec::new();
                let value = meta.value()?;
                let content;
                syn::parenthesized!(content in value);
                while !content.is_empty() {
                    let is_desc = content.peek(Token![-]);
                    if is_desc {
                        content.parse::<Token![-]>()?;
                    }
                    let dim: Ident = content.parse()?;
                    let dir = if is_desc {
                        Direction::Descending
                    } else {
                        Direction::Ascending
                    };
                    priority.push((dim, dir));
                    if !content.is_empty() {
                        content.parse::<Token![,]>()?;
                    }
                }
                result.priority = Some(priority);
                Ok(())
            } else if meta.path.is_ident("concurrency") {
                if result.concurrency.is_some() {
                    return Err(meta.error(
                        "Multiple concurrency keys found in `#[operon(...)]`",
                    ));
                }
                let value = meta.value()?;
                let lit: LitInt = value.parse()?;
                let val = lit.base10_parse::<usize>().map_err(|_| {
                    syn::Error::new(
                        lit.span(),
                        "Invalid concurrency value; expected a positive integer",
                    )
                })?;
                if val == 0 {
                    return Err(syn::Error::new(
                        lit.span(),
                        "Concurrency value of 0 is not allowed",
                    ));
                }
                result.concurrency = Some(PoolSizeSpec::Literal(val));
                Ok(())
            } else if meta.path.is_ident("concurrency_env") {
                if result.concurrency.is_some() {
                    return Err(meta.error(
                        "Multiple concurrency keys found in `#[operon(...)]`",
                    ));
                }
                let value = meta.value()?;
                let ident: Ident = value.parse()?;
                let var_name = ident.to_string();
                // Validate at macro-expansion time if the variable is already in the environment.
                // If not, we skip validation and let the runtime handle it.
                if let Ok(val_str) = std::env::var(&var_name) {
                    match val_str.parse::<usize>() {
                        Err(_) => return Err(syn::Error::new(
                            ident.span(),
                            format!(
                                "Environment variable `{var_name}` is not a valid concurrency value (got \"{val_str}\")"
                            ),
                        )),
                        Ok(0) => return Err(syn::Error::new(
                            ident.span(),
                            format!(
                                "Environment variable `{var_name}` must not be zero for task concurrency"
                            ),
                        )),
                        Ok(_) => {}
                    }
                }
                result.concurrency = Some(PoolSizeSpec::Env(var_name));
                Ok(())
            } else {
                Err(meta.error(
                    "Unknown key in `#[operon(...)]`; accepted keys are: `ord`, `concurrency`, `concurrency_env`",
                ))
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
        assert!(parsed.operon_attrs.priority.is_none());
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
        let priority = parsed
            .operon_attrs
            .priority
            .expect("Priority should be present");
        assert_eq!(priority.len(), 2);
        assert_eq!(priority[0].0.to_string(), "k");
        assert_eq!(priority[0].1, Direction::Descending);
        assert_eq!(priority[1].0.to_string(), "i");
        assert_eq!(priority[1].1, Direction::Ascending);
    }

    #[test]
    fn test_job_decl_concurrency_literal() {
        let input = "#[operon(concurrency=8)] E = epsilon(B<j>, D<j>) for i, k;";
        let parsed: JobDecl = parse_str(input).expect("Failed to parse");
        assert_eq!(
            parsed.operon_attrs.concurrency,
            Some(PoolSizeSpec::Literal(8))
        );
        assert!(parsed.pool.is_none());
    }

    #[test]
    fn test_job_decl_concurrency_env() {
        let input = "#[operon(concurrency_env=JOB_CONCURRENCY)] E = epsilon(B<j>, D<j>) for i, k;";
        let parsed: JobDecl = parse_str(input).expect("Failed to parse");
        assert_eq!(
            parsed.operon_attrs.concurrency,
            Some(PoolSizeSpec::Env("JOB_CONCURRENCY".to_string()))
        );
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
            "#[operon(concurrency=0)] E = job() for i;",         // Zero concurrency
            "#[operon(concurrency=8)] E = job() for(4) i;",      // Conflict with for(N)
            "#[operon(concurrency=8)] #[operon(concurrency_env=X)] E = job() for i;", /* Duplicate concurrency */
        ];
        let results = malformed_inputs
            .into_iter()
            .map(parse_str::<JobDecl>)
            .collect::<Vec<_>>();
        assert!(results.iter().all(|result| result.is_err()));
    }
}
