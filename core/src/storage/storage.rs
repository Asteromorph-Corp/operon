use async_trait::async_trait;

use crate::storage::StorageError;

/// # OperonStorage trait
///
/// This trait contains the storage operations Operon will use.
/// Implement this trait to provide a custom storage backend.
///
/// Notes:
///
/// * All functions are async methods and must return an `anyhow::Result`.
/// * The `clear` function should clear all data EXCEPT the primary data in the storage.
/// * The `put_*` functions' default behaviour must be to *overwrite* existing data.
///   While this is almost never a problem, choosing not to do so may lead to
///   undefined behaviour in certain pause-and-resume scenarios.
/// * The `get_*` functions must return `None` instead of an error if the data is not found.
/// * The optional `clear_footprint`, `put_footprint` and `get_footprint` functions
///   are used to manipulate the footprint of the data.
///   The footprint is used to verify the integrity of the data on a recovery from
///   previous runs that were gracefully shut down.
///   Provide these functions if you want to support fast progress restorations from
///   graceful stops.
/// * Due to having repeated types in the `OperonService` signatures,
///   the default implementations of the `put_*` and `get_*` functions
///   may cause DB-intensive behaviour.
///   If you wish to minimize the number of DB operations,
///   you can implement the provided `put_all_*` and `get_all_*_over_*` functions.
///   The same rules for the `put_*` and `get_*` functions apply to these as well.
///   Additionally, note that these functions assume
///   that repeated data is sorted by the dimension it is repeated on.
///
/// Please consult the following section for exact function signatures.
///
/// ## Function Signatures
///
/// The functions were parsed as follows:
///
/// ```rust
/// use anyhow::Result;
/// use async_trait::async_trait;
/// use operon::dimension::*;
/// use operon::entity::*;
/// #[async_trait]
/// pub trait OperonStorage {
///     async fn clear(&self) -> Result<()>;
///     async fn put_a(&self, i: I, value: &A) -> Result<()>;
///     async fn get_a(&self, i: I) -> Result<Option<A>>;
///     async fn put_b(&self, i: I, j: J, value: &B) -> Result<()>;
///     async fn get_b(&self, i: I, j: J) -> Result<Option<B>>;
///     async fn put_c(&self, i: I, k: K, value: &C) -> Result<()>;
///     async fn get_c(&self, i: I, k: K) -> Result<Option<C>>;
///     async fn put_d(&self, i: I, j: J, k: K, value: &D) -> Result<()>;
///     async fn get_d(&self, i: I, j: J, k: K) -> Result<Option<D>>;
///     async fn put_e(&self, i: I, k: K, value: &E) -> Result<()>;
///     async fn get_e(&self, i: I, k: K) -> Result<Option<E>>;
///     async fn put_f(&self, i: I, value: &F) -> Result<()>;
///     async fn get_f(&self, i: I) -> Result<Option<F>>;
///     // Optional footprint operations:
///     async fn clear_footprint(&self) -> Result<()>;
///     async fn put_footprint(&self, footprint: &str) -> Result<()>;
///     async fn get_footprint(&self) -> Result<Option<String>>;
///     // Optional batch operations:
///     async fn put_all_b(&self, i: I, values: &[B]) -> Result<()>;
///     async fn put_all_c(&self, i: I, values: &[C]) -> Result<()>;
///     async fn get_all_b_over_j(&self, i: I) -> Result<Vec<B>>;
///     async fn get_all_c_over_k(&self, i: I) -> Result<Vec<C>>;
///     async fn get_all_d_over_j(&self, i: I, k: K) -> Result<Vec<D>>;
///     async fn get_all_e_over_k(&self, i: I) -> Result<Vec<E>>;
/// }
/// ```
#[async_trait]
pub trait OperonStorage: Send + Sync + 'static {
    async fn clear(&self) -> ::anyhow::Result<()>;

    async fn get_footprint(&self) -> Result<Option<String>, StorageError> {
        // This function is no-op by default, disallowing recovery runs if not implemented.
        Ok(None)
    }
    async fn put_footprint(&self, _footprint: &str) -> Result<(), StorageError> {
        // This function is no-op by default, disallowing recovery runs if not implemented.
        Ok(())
    }
    async fn clear_footprint(&self) -> Result<(), StorageError> {
        // This function is no-op by default, disallowing recovery runs if not implemented.
        Ok(())
    }
}
