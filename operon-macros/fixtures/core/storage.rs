/// Generated trait containing the entity accessors that should be implemented for use with Operon.
///
/// Implement it alongside `operon::OperonStorage`, which covers the backend's lifecycle and its run footprint.
/// That implementation declares the `Self::Error` these methods report failure as.
/// A coordinate addresses one entity across the pipeline's dimensions, and `operon::Entity` pairs a coordinate with the value stored there.
///
/// # Required methods
/// ```rust,ignore
/// async fn get_a(&self, coordinate: [usize; 1]) -> StorageResult<Option<A>, Self::Error>
/// async fn put_a(&self, entity: Entity<1, A>) -> StorageResult<(), Self::Error>
/// async fn get_b(&self, coordinate: [usize; 2]) -> StorageResult<Option<B>, Self::Error>
/// async fn put_b(&self, entity: Entity<2, B>) -> StorageResult<(), Self::Error>
/// async fn get_c(&self, coordinate: [usize; 2]) -> StorageResult<Option<C>, Self::Error>
/// async fn put_c(&self, entity: Entity<2, C>) -> StorageResult<(), Self::Error>
/// async fn get_d(&self, coordinate: [usize; 3]) -> StorageResult<Option<D>, Self::Error>
/// async fn put_d(&self, entity: Entity<3, D>) -> StorageResult<(), Self::Error>
/// async fn get_e(&self, coordinate: [usize; 2]) -> StorageResult<Option<E>, Self::Error>
/// async fn put_e(&self, entity: Entity<2, E>) -> StorageResult<(), Self::Error>
/// async fn get_f(&self, coordinate: [usize; 1]) -> StorageResult<Option<F>, Self::Error>
/// async fn put_f(&self, entity: Entity<1, F>) -> StorageResult<(), Self::Error>
/// ```
///
/// # Provided methods
/// Each of these covers a whole range of one entity in a single call.
/// They default to walking the accessors above one entity at a time; override them wherever the backend can serve the range in one query.
/// The return value of `get_all_*` should be ordered by the dimensions they iterate over.
/// ```rust,ignore
/// async fn get_all_b_j(&self, coordinate: [usize; 1]) -> StorageResult<Vec<B>, Self::Error>
/// async fn get_all_c_k(&self, coordinate: [usize; 1]) -> StorageResult<Vec<C>, Self::Error>
/// async fn get_all_d_j(&self, coordinate: [usize; 2]) -> StorageResult<Vec<D>, Self::Error>
/// async fn get_all_e_k(&self, coordinate: [usize; 1]) -> StorageResult<Vec<E>, Self::Error>
/// async fn put_all_a(&self, entity: Entity<0, Vec<A>>) -> StorageResult<(), Self::Error>
/// async fn put_all_b(&self, entity: Entity<1, Vec<B>>) -> StorageResult<(), Self::Error>
/// async fn put_all_c(&self, entity: Entity<1, Vec<C>>) -> StorageResult<(), Self::Error>
/// ```
#[operon::__private::async_trait::async_trait]
pub trait CookingStorage: operon::OperonStorage {
    /// ```rust,ignore
    /// async fn get_a(&self, coordinate: [usize; 1]) -> StorageResult<Option<A>, Self::Error>
    /// ```
    /// Reads the `A` stored at `[i]`, or `None` if that coordinate holds nothing.
    async fn get_a(
        &self,
        coordinate: [usize; 1usize],
    ) -> operon::error::StorageResult<Option<A>, Self::Error>;

    /// ```rust,ignore
    /// async fn put_a(&self, entity: Entity<1, A>) -> StorageResult<(), Self::Error>
    /// ```
    /// Writes the given `A` at its own coordinate `[i]`, replacing whatever is stored there.
    async fn put_a(
        &self,
        entity: operon::Entity<1usize, A>,
    ) -> operon::error::StorageResult<(), Self::Error>;

    /// ```rust,ignore
    /// async fn get_b(&self, coordinate: [usize; 2]) -> StorageResult<Option<B>, Self::Error>
    /// ```
    /// Reads the `B` stored at `[i, j]`, or `None` if that coordinate holds nothing.
    async fn get_b(
        &self,
        coordinate: [usize; 2usize],
    ) -> operon::error::StorageResult<Option<B>, Self::Error>;

    /// ```rust,ignore
    /// async fn put_b(&self, entity: Entity<2, B>) -> StorageResult<(), Self::Error>
    /// ```
    /// Writes the given `B` at its own coordinate `[i, j]`, replacing whatever is stored there.
    async fn put_b(
        &self,
        entity: operon::Entity<2usize, B>,
    ) -> operon::error::StorageResult<(), Self::Error>;

    /// ```rust,ignore
    /// async fn get_c(&self, coordinate: [usize; 2]) -> StorageResult<Option<C>, Self::Error>
    /// ```
    /// Reads the `C` stored at `[i, k]`, or `None` if that coordinate holds nothing.
    async fn get_c(
        &self,
        coordinate: [usize; 2usize],
    ) -> operon::error::StorageResult<Option<C>, Self::Error>;

    /// ```rust,ignore
    /// async fn put_c(&self, entity: Entity<2, C>) -> StorageResult<(), Self::Error>
    /// ```
    /// Writes the given `C` at its own coordinate `[i, k]`, replacing whatever is stored there.
    async fn put_c(
        &self,
        entity: operon::Entity<2usize, C>,
    ) -> operon::error::StorageResult<(), Self::Error>;

    /// ```rust,ignore
    /// async fn get_d(&self, coordinate: [usize; 3]) -> StorageResult<Option<D>, Self::Error>
    /// ```
    /// Reads the `D` stored at `[i, j, k]`, or `None` if that coordinate holds nothing.
    async fn get_d(
        &self,
        coordinate: [usize; 3usize],
    ) -> operon::error::StorageResult<Option<D>, Self::Error>;

    /// ```rust,ignore
    /// async fn put_d(&self, entity: Entity<3, D>) -> StorageResult<(), Self::Error>
    /// ```
    /// Writes the given `D` at its own coordinate `[i, j, k]`, replacing whatever is stored there.
    async fn put_d(
        &self,
        entity: operon::Entity<3usize, D>,
    ) -> operon::error::StorageResult<(), Self::Error>;

    /// ```rust,ignore
    /// async fn get_e(&self, coordinate: [usize; 2]) -> StorageResult<Option<E>, Self::Error>
    /// ```
    /// Reads the `E` stored at `[i, k]`, or `None` if that coordinate holds nothing.
    async fn get_e(
        &self,
        coordinate: [usize; 2usize],
    ) -> operon::error::StorageResult<Option<E>, Self::Error>;

    /// ```rust,ignore
    /// async fn put_e(&self, entity: Entity<2, E>) -> StorageResult<(), Self::Error>
    /// ```
    /// Writes the given `E` at its own coordinate `[i, k]`, replacing whatever is stored there.
    async fn put_e(
        &self,
        entity: operon::Entity<2usize, E>,
    ) -> operon::error::StorageResult<(), Self::Error>;

    /// ```rust,ignore
    /// async fn get_f(&self, coordinate: [usize; 1]) -> StorageResult<Option<F>, Self::Error>
    /// ```
    /// Reads the `F` stored at `[i]`, or `None` if that coordinate holds nothing.
    async fn get_f(
        &self,
        coordinate: [usize; 1usize],
    ) -> operon::error::StorageResult<Option<F>, Self::Error>;

    /// ```rust,ignore
    /// async fn put_f(&self, entity: Entity<1, F>) -> StorageResult<(), Self::Error>
    /// ```
    /// Writes the given `F` at its own coordinate `[i]`, replacing whatever is stored there.
    async fn put_f(
        &self,
        entity: operon::Entity<1usize, F>,
    ) -> operon::error::StorageResult<(), Self::Error>;

    /// ```rust,ignore
    /// async fn get_all_b_j(&self, coordinate: [usize; 1]) -> StorageResult<Vec<B>, Self::Error>
    /// ```
    /// Reads every `B` stored at `[i, j]` over `j`, counting that dimension up from `0` and stopping at the first coordinate that holds nothing.
    /// Defaults to walking `get_b` one entity at a time.
    async fn get_all_b_j(
        &self,
        [i]: [usize; 1usize],
    ) -> operon::error::StorageResult<Vec<B>, Self::Error> {
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
    /// async fn get_all_c_k(&self, coordinate: [usize; 1]) -> StorageResult<Vec<C>, Self::Error>
    /// ```
    /// Reads every `C` stored at `[i, k]` over `k`, counting that dimension up from `0` and stopping at the first coordinate that holds nothing.
    /// Defaults to walking `get_c` one entity at a time.
    async fn get_all_c_k(
        &self,
        [i]: [usize; 1usize],
    ) -> operon::error::StorageResult<Vec<C>, Self::Error> {
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
    /// async fn get_all_d_j(&self, coordinate: [usize; 2]) -> StorageResult<Vec<D>, Self::Error>
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
    /// async fn get_all_e_k(&self, coordinate: [usize; 1]) -> StorageResult<Vec<E>, Self::Error>
    /// ```
    /// Reads every `E` stored at `[i, k]` over `k`, counting that dimension up from `0` and stopping at the first coordinate that holds nothing.
    /// Defaults to walking `get_e` one entity at a time.
    async fn get_all_e_k(
        &self,
        [i]: [usize; 1usize],
    ) -> operon::error::StorageResult<Vec<E>, Self::Error> {
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

    /// ```rust,ignore
    /// async fn put_all_a(&self, entity: Entity<0, Vec<A>>) -> StorageResult<(), Self::Error>
    /// ```
    /// Writes a whole run of `A` at `[i]`, taking `i` from each value's position in `entity.value`.
    /// Defaults to walking `put_a` one entity at a time.
    async fn put_all_a(
        &self,
        entity: operon::Entity<0usize, Vec<A>>,
    ) -> operon::error::StorageResult<(), Self::Error> {
        let [] = entity.coordinate;
        for (i, value) in entity.value.into_iter().enumerate() {
            let entity_single = operon::Entity {
                coordinate: [i],
                value,
            };
            self.put_a(entity_single).await?;
        }
        Ok(())
    }

    /// ```rust,ignore
    /// async fn put_all_b(&self, entity: Entity<1, Vec<B>>) -> StorageResult<(), Self::Error>
    /// ```
    /// Writes a whole run of `B` at `[i, j]`, taking `j` from each value's position in `entity.value`.
    /// Defaults to walking `put_b` one entity at a time.
    async fn put_all_b(
        &self,
        entity: operon::Entity<1usize, Vec<B>>,
    ) -> operon::error::StorageResult<(), Self::Error> {
        let [i] = entity.coordinate;
        for (j, value) in entity.value.into_iter().enumerate() {
            let entity_single = operon::Entity {
                coordinate: [i, j],
                value,
            };
            self.put_b(entity_single).await?;
        }
        Ok(())
    }

    /// ```rust,ignore
    /// async fn put_all_c(&self, entity: Entity<1, Vec<C>>) -> StorageResult<(), Self::Error>
    /// ```
    /// Writes a whole run of `C` at `[i, k]`, taking `k` from each value's position in `entity.value`.
    /// Defaults to walking `put_c` one entity at a time.
    async fn put_all_c(
        &self,
        entity: operon::Entity<1usize, Vec<C>>,
    ) -> operon::error::StorageResult<(), Self::Error> {
        let [i] = entity.coordinate;
        for (k, value) in entity.value.into_iter().enumerate() {
            let entity_single = operon::Entity {
                coordinate: [i, k],
                value,
            };
            self.put_c(entity_single).await?;
        }
        Ok(())
    }
}
