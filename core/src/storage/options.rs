use secrecy::SecretString;

pub struct StorageOptions {
    pub database_uri: SecretString,
    pub pool_size: usize,
    pub keepalives_idle: std::time::Duration,
    pub keepalives_interval: std::time::Duration,
    pub schema: Option<String>,
}

impl StorageOptions {
    pub fn new(database_uri: impl Into<String>) -> Self {
        Self {
            database_uri: SecretString::from(database_uri.into()),
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
        if let Some(ref s) = schema {
            if !s.chars().all(|c| c.is_alphanumeric() || c == '_') {
                panic!("Schema name can only contain alphanumeric characters and underscores");
            } else if s.is_empty() {
                panic!("Schema name cannot be empty");
            }
        }
        self.schema = schema;
        self
    }
}
