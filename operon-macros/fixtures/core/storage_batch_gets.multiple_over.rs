/// ```rust,ignore
/// async fn get_all_c_j(coordinate: [usize; 0]) -> StorageResult<Vec<C>, Self::Error>
/// ```
/// Reads every `C` stored at `[j]` over `j`, counting that dimension up from `0` and stopping at the first coordinate that holds nothing.
/// Defaults to walking `get_c` one entity at a time.
async fn get_all_c_j(&self, []: [usize; 0usize]) -> operon::error::StorageResult<Vec<C>, Self::Error> {
    let final_results = {
        let mut results_0 = Vec::new();
        let mut j = 0usize;
        while let Some(value) = self.get_c([j]).await? {
            results_0.push(value);
            j += 1;
        }
        (!results_0.is_empty()).then_some(results_0)
    };
    Ok(final_results.unwrap_or_default())
}

/// ```rust,ignore
/// async fn get_all_d_jk(coordinate: [usize; 1]) -> StorageResult<Vec<Vec<D>>, Self::Error>
/// ```
/// Reads every `D` stored at `[i, j, k]` over `j`, `k`, counting those dimensions up from `0` and stopping at the first coordinate that holds nothing.
/// Defaults to walking `get_d` one entity at a time.
async fn get_all_d_jk(
    &self,
    [i]: [usize; 1usize],
) -> operon::error::StorageResult<Vec<Vec<D>>, Self::Error> {
    let final_results = {
        let mut results_0 = Vec::new();
        let mut j = 0usize;
        while let Some(value) = {
            let mut results_1 = Vec::new();
            let mut k = 0usize;
            while let Some(value) = self.get_d([i, j, k]).await? {
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
