use secrecy::SecretString;

pub struct MetaStorageOptions {
    pub(crate) database_uri: SecretString,
    pub(crate) pool_size: usize,
    pub(crate) keepalives_idle: std::time::Duration,
    pub(crate) keepalives_interval: std::time::Duration,
    pub(crate) schema: Option<String>,
}
