use super::entity_decl::EntityDecl;
use syn::{
    Ident, LitInt, Token,
    parse::{Parse, ParseStream},
    spanned::Spanned,
    token::Paren,
};

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

        Ok(())
    }
}
impl Parse for JobDecl {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let start = input.span();
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
            _span,
        };
        job_decl.validate()?;
        Ok(job_decl)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_str;

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
    fn test_job_decl_malformed() {
        let malformed_inputs = [
            "Entity = JobName() for i",                          // Missing semicolon
            "Entity = JobName() for(8 i, j;",                    // Missing closing parenthesis
            "Entity = JobName() for;",                           // Redundant for
            "Entity<i, j> = JobName() for(8) i, j;",             // Too many dimensions
            "Entity = JobName for(8) i;",                        // Missing argument parens
            "Entity = JobName() for i, j k;",                    // Missing comma
            "Entity = JobName() for(8) i, j, k; SomeExtraToken", // Extra token after semicolon
        ];
        let results = malformed_inputs
            .into_iter()
            .map(parse_str::<JobDecl>)
            .collect::<Vec<_>>();
        assert!(results.iter().all(|result| result.is_err()));
        // for result in results {
        //     println!("Result: {result:?}");
        // }
    }
}
