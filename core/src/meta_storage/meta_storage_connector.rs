use crate::meta_storage::{MetaClient, MetaStorage};

pub trait MetaStorageConnector {
    type MetaSto<'a>: MetaStorage + Send + Sync;

    fn connect<'a>(client: impl Into<MetaClient<'a>>, schema: Option<&'a str>)
    -> Self::MetaSto<'a>;
}
