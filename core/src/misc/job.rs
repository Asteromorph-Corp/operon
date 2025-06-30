use std::fmt::Debug;

pub trait Job: Debug + Clone + Send + Sync + 'static {}

pub trait JobEnum: Debug + Clone + Send + Sync + 'static {}
