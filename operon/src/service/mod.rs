pub use operon_macros::OperonService;

use crate::schema::{JobEnum, ResolutionEnum, TicketEnum};

/// A type-carrier trait for validating the service type.
/// This trait **should not** be implemented manually; the derive macro
/// [`#[derive(OperonService)]`](derive@crate::OperonService) is the only correct way to implement
/// it.
/// The derive takes `#[operon(error = MyError)]`, `#[operon(defined_at = "path")]`, and
/// `#[operon(crate = "path")]`.
///
/// Along with the `{PipelineName}Service` trait that [`define_operon!`](crate::define_operon)
/// generates, this trait provides the scheduler with the information it needs to execute the
/// pipeline.
pub trait OperonService: Send + Sync + 'static {
    /// The error a service method may return.
    /// Carried by [`SchedulerError::UserError`](crate::error::SchedulerError::UserError), which the
    /// UI consumes rather than returning to the caller.
    /// Defaults to [`UserError`](crate::operon::UserError) (alias of `Box<dyn std::error::Error +
    /// Send + Sync>`).
    ///
    /// To change this type, use `#[operon(error = MyError)]` on your service type with
    /// `#[derive(OperonService)]`.
    type Error: std::fmt::Debug + std::fmt::Display + Send + Sync + 'static;

    /// The enum type representing a job of any task in the pipeline.
    type JobEnum: JobEnum;

    /// The enum type representing a resolution of any dimension in the pipeline.
    type ResolutionEnum: ResolutionEnum;

    /// The enum type representing a ticket of any task in the pipeline.
    type TicketEnum: TicketEnum;
}
