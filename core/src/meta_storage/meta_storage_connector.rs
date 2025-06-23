use crate::meta_storage::{MetaClient, MetaStorage};

pub trait MetaStorageConnector: Send + Sync + 'static {
    type MetaSto<'a>: MetaStorage;

    fn connect<'a>(client: impl Into<MetaClient<'a>>, schema: Option<&'a str>)
    -> Self::MetaSto<'a>;
}
