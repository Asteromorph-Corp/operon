use std::fmt::Debug;

pub trait Resolution: Debug + Clone + Send + Sync + 'static {}

impl Resolution for () {}
