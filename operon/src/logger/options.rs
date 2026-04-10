pub struct LoggerOptions {
    pub level: tracing::Level,
    pub buffer_size: usize,
    pub dump: Option<String>,
}
