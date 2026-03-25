#[derive(Debug, Clone, Copy)]
pub struct UiOptions {
    pub log_buffer_size: usize,
    pub mode: UiMode,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiMode {
    #[default]
    Interactive,
    Headless,
}
