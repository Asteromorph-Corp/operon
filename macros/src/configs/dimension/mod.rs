#[allow(clippy::module_inception)]
mod dimension;
pub use dimension::*;

mod init_resolution;
pub use init_resolution::*;

mod clear_resolution;
pub use clear_resolution::*;

mod get_resolution;
pub use get_resolution::*;

mod put_resolution;
pub use put_resolution::*;
