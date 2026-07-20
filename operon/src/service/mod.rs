pub use operon_macros::OperonService;

use crate::schema::{JobEnum, ResolutionEnum, TicketEnum};

pub trait OperonService: Send + Sync + 'static {
    /// The error a service method may return.
    /// Surfaces through [`OperonError::User`](crate::operon::OperonError::User).
    /// Defaults to [`UserError`](crate::operon::UserError) (alias of `Box<dyn std::error::Error + Send +
    /// Sync>`).
    ///
    /// To change this type, use `#[operon(error = "MyError")]` on your service type with
    /// `#[derive(OperonService)]`.
    type Error: std::fmt::Debug + std::fmt::Display + Send + Sync + 'static;

    type JobEnum: JobEnum;
    type ResolutionEnum: ResolutionEnum;
    type TicketEnum: TicketEnum;
}
