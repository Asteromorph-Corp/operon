use std::borrow::Cow;

pub struct LogOptions {
    pub level: log::Level,
    pub buffer_size: usize,
    pub dump: Option<Cow<'static, str>>,
}
