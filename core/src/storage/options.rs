pub struct StorageOptions {
    pub database_uri: String,
    pub pool_size: usize,
    pub keepalives_idle: std::time::Duration,
    pub keepalives_interval: std::time::Duration,
    pub schema: Option<String>,
}

impl StorageOptions {
    pub fn new(database_uri: String) -> Self {
        Self {
            database_uri,
            pool_size: 16,
            keepalives_idle: std::time::Duration::from_secs(60),
            keepalives_interval: std::time::Duration::from_secs(30),
            schema: None,
        }
    }

    pub fn with_pool_size(mut self, size: usize) -> Self {
        self.pool_size = size;
        self
    }

    pub fn with_keepalives_idle(mut self, duration: std::time::Duration) -> Self {
        self.keepalives_idle = duration;
        self
    }

    pub fn with_keepalives_interval(mut self, duration: std::time::Duration) -> Self {
        self.keepalives_interval = duration;
        self
    }

    pub fn with_schema(mut self, schema: Option<String>) -> Self {
        self.schema = schema;
        self
    }
}
