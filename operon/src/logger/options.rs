pub struct LoggerOptions {
    pub level: log::Level,
    pub buffer_size: usize,
    pub dump: Option<String>,
}
