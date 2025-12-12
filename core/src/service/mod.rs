pub use operon_macros::OperonService;

use crate::schema::{JobEnum, ResolutionEnum, TicketEnum};

pub trait OperonService: Send + Sync + 'static {
    type JobEnum: JobEnum;
    type ResolutionEnum: ResolutionEnum;
    type TicketEnum: TicketEnum;
}
