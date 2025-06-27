use std::fmt::Debug;

pub trait Job: Debug + Clone + Send + Sync + 'static {}
