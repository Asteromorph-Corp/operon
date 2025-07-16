use syn::parse_quote;

use crate::JobConfig;

pub(super) fn fn_pool_size(job: &JobConfig) -> syn::ImplItemFn {
    let pool_size = job.pool_size;

    parse_quote! {
        fn pool_size(&self) -> usize {
            #pool_size
        }
    }
}
