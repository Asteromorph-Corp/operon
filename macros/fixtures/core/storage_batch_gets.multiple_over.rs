async fn get_all_b_over_j(&self, i: schema::IDim) -> Result<Vec<B>, operon::storage::StorageError> {
    let final_results = {
        let mut results_0 = Vec::new();
        let mut j = 0usize;
        while let Some(value) = self.get_b(i, j.into()).await? {
            results_0.push(value);
            j += 1;
        }
        (!results_0.is_empty()).then_some(results_0)
    };
    Ok(final_results.unwrap_or_default())
}

async fn get_all_c_over_jk(
    &self,
    i: schema::IDim,
) -> Result<Vec<Vec<C>>, operon::storage::StorageError> {
    let final_results = {
        let mut results_0 = Vec::new();
        let mut j = 0usize;
        while let Some(value) = {
            let mut results_1 = Vec::new();
            let mut k = 0usize;
            while let Some(value) = self.get_c(i, j.into(), k.into()).await? {
                results_1.push(value);
                k += 1;
            }
            (!results_1.is_empty()).then_some(results_1)
        } {
            results_0.push(value);
            j += 1;
        }
        (!results_0.is_empty()).then_some(results_0)
    };
    Ok(final_results.unwrap_or_default())
}
