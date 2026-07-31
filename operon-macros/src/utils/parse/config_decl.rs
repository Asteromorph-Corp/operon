use syn::parse::{Parse, ParseStream};
use syn::token::Brace;
use syn::{Ident, Token};

use super::job_decl::JobDecl;

#[derive(Debug)]
pub(super) struct ConfigDecl {
    pub(super) service_id: Ident,
    pub(super) _eq_token: Token![=],
    pub(super) _braces_token: Brace,
    pub(super) jobs: Vec<JobDecl>,
    pub(super) _span: proc_macro2::Span,
}
impl ConfigDecl {
    pub fn validate(&self) -> syn::Result<()> {
        if self.jobs.is_empty() {
            return Err(syn::Error::new(
                self._span,
                "Configuration must contain at least one task",
            ));
        }
        Ok(())
    }
}
impl Parse for ConfigDecl {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let start = input.span();
        let service_id: Ident = input.parse()?;
        let eq_token: Token![=] = input.parse()?;
        let jobs_content;
        let braces_token: Brace = syn::braced!(jobs_content in input);
        let mut jobs = Vec::new();
        while !jobs_content.is_empty() {
            let job: JobDecl = jobs_content.parse()?;
            jobs.push(job);
        }
        let end = input.span();
        let _span = start.join(end).unwrap_or(start);
        let config_decl = ConfigDecl {
            service_id,
            _eq_token: eq_token,
            _braces_token: braces_token,
            jobs,
            _span,
        };
        config_decl.validate()?;
        Ok(config_decl)
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_str;

    use super::*;

    #[test]
    fn test_config_decl() {
        let input = "cooking = {
            A<i> = alpha();
            B<j> = beta(A) for(8) i;
            C<k> = gamma(A) for(8) i;
            D    = delta(A, B, C) for(4) i, j, k;
            E    = epsilon(B<j>, D<j>) for(4) i, k;
            F    = zeta(C<k>, E<k>) for i;
        }";
        let parsed: ConfigDecl = parse_str(input).expect("Failed to parse");
        assert_eq!(parsed.service_id.to_string(), "cooking");
        assert_eq!(parsed.jobs.len(), 6);
    }

    #[test]
    fn test_config_decl_malformed() {
        let malformed_inputs = [
            "cooking = |A<i>, B<i>| { C<j> = gamma(A, B) for i; }", // Double primary entity
            "cooking = || { A<i> = spawn_primary(); }",             // Missing primary entity
            "cooking |A<i>| { B<j> = beta(A) for(8) i; }",          // Missing equal sign
            "cooking = |A<i>| {}",                                  // Empty job block
            "cooking = |A<i, j>| { B<k> = beta(A) for(8) i, j; }",  /* Multiple dimensions in
                                                                     * primary entity */
            "cooking = |A| { B<i> = beta(A); }", // Missing primary entity dimension
        ];
        let results = malformed_inputs
            .iter()
            .map(|input| parse_str::<ConfigDecl>(input))
            .collect::<Vec<_>>();
        assert!(results.iter().all(|result| result.is_err()));
        // for result in results {
        //     println!("Result: {result:?}");
        // }
    }
}
