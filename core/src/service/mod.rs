use crate::schema_base::{JobEnum, ResolutionEnum};

pub trait OperonService: Send + Sync + 'static {
    type JobEnum: JobEnum;
    type ResolutionEnum: ResolutionEnum;
}
