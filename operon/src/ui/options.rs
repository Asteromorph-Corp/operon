#[derive(Debug, Clone, Copy)]
pub(crate) struct UiOptions {
    pub log_buffer_size: usize,
    pub mode: UiMode,
}

/// Frontend mode to execute the Operon engine.
///
/// Available options are:
///
/// - [`Self::Interactive`] (default)
/// - [`Self::Headless`]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum UiMode {
    /// Open an interactive TUI in the terminal to execute the pipeline by commands.
    #[default]
    Interactive,
    /// Run the engine without a TUI.
    ///
    /// Operon will automatically discard previous data, start the pipeline, and run up to a
    /// successful termination or an error.
    /// Logs will be drained to `stderr` instead.
    Headless,
}
