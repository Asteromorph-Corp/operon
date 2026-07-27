/// ```rust,ignore
/// async fn get_all_b_j(coordinate: [usize; 1]) -> StorageResult<Vec<B>, Self::Error>
/// ```
/// Reads every `B` stored at `[i, j]` over `j`, counting that dimension up from `0` and stopping at the first coordinate that holds nothing.
/// Defaults to walking `get_b` one entity at a time.
async fn get_all_b_j(&self, [i]: [usize; 1usize]) -> operon::error::StorageResult<Vec<B>, Self::Error> {
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

/// ```rust,ignore
/// async fn get_all_c_k(coordinate: [usize; 1]) -> StorageResult<Vec<C>, Self::Error>
/// ```
/// Reads every `C` stored at `[i, k]` over `k`, counting that dimension up from `0` and stopping at the first coordinate that holds nothing.
/// Defaults to walking `get_c` one entity at a time.
async fn get_all_c_k(&self, [i]: [usize; 1usize]) -> operon::error::StorageResult<Vec<C>, Self::Error> {
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

/// ```rust,ignore
/// async fn get_all_d_j(coordinate: [usize; 2]) -> StorageResult<Vec<D>, Self::Error>
/// ```
/// Reads every `D` stored at `[i, j, k]` over `j`, counting that dimension up from `0` and stopping at the first coordinate that holds nothing.
/// Defaults to walking `get_d` one entity at a time.
async fn get_all_d_j(
    &self,
    [i, k]: [usize; 2usize],
) -> operon::error::StorageResult<Vec<D>, Self::Error> {
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

/// ```rust,ignore
/// async fn get_all_e_k(coordinate: [usize; 1]) -> StorageResult<Vec<E>, Self::Error>
/// ```
/// Reads every `E` stored at `[i, k]` over `k`, counting that dimension up from `0` and stopping at the first coordinate that holds nothing.
/// Defaults to walking `get_e` one entity at a time.
async fn get_all_e_k(&self, [i]: [usize; 1usize]) -> operon::error::StorageResult<Vec<E>, Self::Error> {
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
