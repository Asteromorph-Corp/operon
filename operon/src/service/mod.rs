pub use operon_macros::OperonService;

use crate::schema::{JobEnum, ResolutionEnum, TicketEnum};

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

    type JobEnum: JobEnum;
    type ResolutionEnum: ResolutionEnum;
    type TicketEnum: TicketEnum;
}
