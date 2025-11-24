async fn get_all_b_over_j(
    &self,
    [i]: [usize; 1usize],
) -> Result<Vec<B>, operon::storage::StorageError> {
    let final_results = {
        let mut results_0 = Vec::new();
        let mut j = 0usize;
        while let Some(value) = self.get_b([i, j]).await? {
            results_0.push(value);
            j += 1;
        }
        (!results_0.is_empty()).then_some(results_0)
    };
    Ok(final_results.unwrap_or_default())
}

async fn get_all_c_over_k(
    &self,
    [i]: [usize; 1usize],
) -> Result<Vec<C>, operon::storage::StorageError> {
    let final_results = {
        let mut results_0 = Vec::new();
        let mut k = 0usize;
        while let Some(value) = self.get_c([i, k]).await? {
            results_0.push(value);
            k += 1;
        }
        (!results_0.is_empty()).then_some(results_0)
    };
    Ok(final_results.unwrap_or_default())
}

async fn get_all_d_over_j(
    &self,
    [i, k]: [usize; 2usize],
) -> Result<Vec<D>, operon::storage::StorageError> {
    let final_results = {
        let mut results_0 = Vec::new();
        let mut j = 0usize;
        while let Some(value) = self.get_d([i, j, k]).await? {
            results_0.push(value);
            j += 1;
        }
        (!results_0.is_empty()).then_some(results_0)
    };
    Ok(final_results.unwrap_or_default())
}

async fn get_all_e_over_k(
    &self,
    [i]: [usize; 1usize],
) -> Result<Vec<E>, operon::storage::StorageError> {
    let final_results = {
        let mut results_0 = Vec::new();
        let mut k = 0usize;
        while let Some(value) = self.get_e([i, k]).await? {
            results_0.push(value);
            k += 1;
        }
        (!results_0.is_empty()).then_some(results_0)
    };
    Ok(final_results.unwrap_or_default())
}
