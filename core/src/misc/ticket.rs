use crate::{
    meta_storage::{MetaClient, MetaStorageError},
    misc::{Job, Resolution},
};

/// Trait that represents tickets for the jobs.
#[::async_trait::async_trait]
pub trait Ticket: std::fmt::Debug + Default + Clone + Sized + Send + Sync + 'static {
    type Job: Job;
    type Resolution: Resolution;

    /// Brand-new ticket, with none of the dimensions resolved.
    /// Return the ticket that should be present at startup time.
    fn new() -> Self {
        Self::default()
    }

    /// Fetch the required dependency count for this ticket.
    /// If the quota is not yet known, return `None`.
    ///
    /// This function is async because it may need to access the fact storage
    /// to get the dependency count.
    ///
    /// The dependency quota for each job is computed as the total number of upstream jobs,
    /// where for each type of upstream job, the quota is a sum of products.
    /// As pseudocode:
    ///
    /// ```lua
    /// --[[ "Sink" dimensions are dimensions with no outdegrees,
    ///     i.e. no other relevant dimensions depend on them. ]]
    /// quota = 0
    /// for parent_coordinates in non_sink_dimensions_valid_coordinates do
    ///   product = 1
    ///   for dimension in sink_dimensions do
    ///     -- We call the fact storage to get this upper_bound here
    ///     product *= dimension.upper_bound(parent_coordinates)
    ///   end
    ///   quota += product
    /// end
    /// print(quota)
    /// ```
    async fn get_dependency_quota(
        &self,
        client: MetaClient<'_>,
    ) -> Result<Option<usize>, MetaStorageError>;

    /// Raise the dependency count of this ticket by one,
    /// and compute if all dependencies are done.
    /// Return the updated ticket.
    ///
    /// `deps_done` is made true if and only if:
    /// * the dependency quota has become known,
    /// * and the dependency count is equal to the required dependency count.
    async fn raise_dependency_count(
        &mut self,
        client: MetaClient<'_>,
    ) -> Result<(), MetaStorageError>;

    /// Whether this ticket is ready to run,
    /// i.e. whether all dependencies are done and the job is fully resolved.
    fn is_ready(&self) -> bool;

    /// Whether this ticket is fully resolved.
    /// Note that this should go through the masked dimensions,
    /// instead of relying on the precomputed values.
    fn is_resolved(&self) -> bool;

    /// The job that this ticket represents.
    /// If the dimensions or dependencies are not fully resolved, the job will be `None`.
    fn resolve(&self) -> Option<Self::Job>;

    // /// Given a dimension resolution, consume this ticket and return the updated tickets.
    // /// It is an error if the given dimension is irrelevant to this ticket.
    // fn explode(self, resolution: Self::Resolution) -> Result<Vec<Self>, MetaStorageError>;

    /// Convert into a string that represents the SQL parameters for this ticket,
    /// in the format that can be used in an `INSERT` statement.
    ///
    /// Formatted as "(value,value,'value',\[...\])".
    fn to_sql_insert_params(&self) -> Result<String, MetaStorageError>;

    /// Convert into a CSV string that represents the SQL parameters for this ticket,
    /// in the format that can be used in a `COPY` statement.
    ///
    /// Formatted as "value,value,value,\[...\]\n".
    fn to_sql_copy_params(&self) -> Result<String, MetaStorageError>;

    /// Convert a SQL row into this ticket.
    fn from_sql_row(row: &::tokio_postgres::Row) -> Result<Self, MetaStorageError>;
}
