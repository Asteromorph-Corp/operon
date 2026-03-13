#[derive(Debug, Clone, Copy)]
pub struct UiOptions {
    pub log_buffer_size: usize,
    pub mode: UiMode,
}

impl Default for UiOptions {
    fn default() -> Self {
        Self {
            log_buffer_size: 1024,
            mode: UiMode::default(),
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiMode {
    #[default]
    Interactive,
    Headless,
}
