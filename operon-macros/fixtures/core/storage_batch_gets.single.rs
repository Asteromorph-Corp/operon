/// ```rust,ignore
/// async fn get_all_a_i(&self, coordinate: [usize; 0]) -> StorageResult<Vec<A>, Self::Error>
/// ```
/// Reads every `A` stored at `[i]` over `i`, counting that dimension up from `0` and stopping at the first coordinate that holds nothing.
/// Defaults to walking `get_a` one entity at a time.
async fn get_all_a_i(
    &self,
    []: [usize; 0usize],
) -> operon::error::StorageResult<Vec<A>, Self::Error> {
    let final_results = {
        let mut results_0 = Vec::new();
        let mut i = 0usize;
        while let Some(value) = self.get_a([i]).await? {
            results_0.push(value);
            i += 1;
        }
        (!results_0.is_empty()).then_some(results_0)
    };
    Ok(final_results.unwrap_or_default())
}
