mod init_resolution;
pub use init_resolution::*;

mod clear_resolution;
pub use clear_resolution::*;

mod get_resolution;
pub use get_resolution::*;

mod put_resolution;
pub use put_resolution::*;

impl crate::configs::DimensionConfig {
    pub fn init_resolution_query(&self) -> InitResolutionQuery<'_> {
        InitResolutionQuery(self)
    }

    pub fn clear_resolution_query(&self) -> ClearResolutionQuery<'_> {
        ClearResolutionQuery(self)
    }

    pub fn get_resolution_query(&self) -> GetResolutionQuery<'_> {
        GetResolutionQuery(self)
    }

    pub fn put_resolution_query(&self) -> PutResolutionQuery<'_> {
        PutResolutionQuery(self)
    }
}
