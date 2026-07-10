use crate::logger::LoggerOptions;
use crate::meta_storage::{MetaBackendOptions, PsqlMetaStorageOptions};
use crate::scheduler::SchedulerOptions;
use crate::ui::{UiMode, UiOptions};

pub struct OperonOptions {
    // UI options
    pub(crate) ui_mode: UiMode,
    // Scheduler options
    pub(crate) internal_channel_size: usize,
    // Meta storage options
    pub(crate) meta_storage_backend: MetaBackendOptions,
    // Log options
    pub(crate) log_level: tracing::Level,
    pub(crate) log_buffer_size: usize,
    pub(crate) log_dump: Option<String>,
}

impl OperonOptions {
    /// Creates options for an Operon backed by a Postgres metadata store at `meta_storage_uri`.
    ///
    /// # Stability
    ///
    /// This is a **Postgres-specific convenience** that predates the backend-agnostic
    /// [`MetaBackendOptions`]. It exists because Postgres is currently the only backend. Once a
    /// second backend lands, this constructor will be **renamed** to make the Postgres assumption
    /// explicit (e.g. `from_psql_uri`) — a breaking change. Code that wants to survive that
    /// transition untouched should construct options through [`OperonOptions::from_backend`]
    /// instead, passing a [`MetaBackendOptions`] explicitly.
    pub fn new(meta_storage_uri: impl Into<String>) -> Self {
        Self::from_backend(MetaBackendOptions::psql(meta_storage_uri))
    }

    /// Creates options for an Operon whose metadata lives in the given backend.
    ///
    /// All other settings start at their defaults and can be overridden with the `with_*` builder
    /// methods. Backend-specific tuning (pool size, schema, keepalives for Postgres) lives inside
    /// the [`MetaBackendOptions`] variant itself.
    pub fn from_backend(backend: MetaBackendOptions) -> Self {
        Self {
            ui_mode: UiMode::Interactive,
            internal_channel_size: 1024,
            meta_storage_backend: backend,
            log_level: tracing::Level::INFO,
            log_buffer_size: 1024,
            log_dump: None,
        }
    }

    pub fn with_ui_mode(mut self, mode: UiMode) -> Self {
        self.ui_mode = mode;
        self
    }

    pub fn with_internal_channel_size(mut self, size: usize) -> Self {
        self.internal_channel_size = size;
        self
    }

    // --- Postgres-specific metadata knobs ---
    //
    // These delegate into the `Psql` variant of the configured backend, tuning its
    // `PsqlMetaStorageOptions`. Postgres is the only backend today, so they always apply.
    //
    // Planned (with the breaking new-backend change): backend selection will yield a specialized
    // options type, with fields shared across backends exposed via macro-generated builders and
    // backend-specific fields present only on the backends that have them — so setting a knob a
    // backend lacks becomes a compile-time error. `map_psql` is the interim stand-in.

    pub fn with_meta_storage_pool_size(self, size: u32) -> Self {
        self.map_psql(|psql| psql.with_pool_size(size))
    }

    pub fn with_meta_storage_schema(self, schema: impl Into<String>) -> Self {
        self.map_psql(|psql| psql.with_schema(schema))
    }

    pub fn with_meta_storage_keepalives_idle(self, duration: std::time::Duration) -> Self {
        self.map_psql(|psql| psql.with_keepalives_idle(duration))
    }

    pub fn with_meta_storage_keepalives_interval(self, duration: std::time::Duration) -> Self {
        self.map_psql(|psql| psql.with_keepalives_interval(duration))
    }

    /// Applies a builder step to the configured backend's [`PsqlMetaStorageOptions`].
    ///
    /// Interim helper for the Postgres-specific `with_meta_storage_*` setters; see the note above
    /// them for the intended specialized-options evolution.
    fn map_psql(
        mut self,
        f: impl FnOnce(PsqlMetaStorageOptions) -> PsqlMetaStorageOptions,
    ) -> Self {
        match self.meta_storage_backend {
            MetaBackendOptions::Psql(psql) => {
                self.meta_storage_backend = MetaBackendOptions::Psql(f(psql));
            }
        }
        self
    }

    pub fn with_log_level(mut self, level: tracing::Level) -> Self {
        self.log_level = level;
        self
    }

    pub fn with_log_buffer_size(mut self, size: usize) -> Self {
        self.log_buffer_size = size;
        self
    }

    pub fn with_log_dump(mut self, dump: impl Into<String>) -> Self {
        self.log_dump = Some(dump.into());
        self
    }

    pub(crate) fn split(self) -> (UiOptions, SchedulerOptions, LoggerOptions) {
        let ui_options = UiOptions {
            mode: self.ui_mode,
            log_buffer_size: self.log_buffer_size,
        };
        let scheduler_options = SchedulerOptions {
            internal_channel_size: self.internal_channel_size,
            ui_mode: self.ui_mode,
            backend: self.meta_storage_backend,
        };
        let log_options = LoggerOptions {
            level: self.log_level,
            buffer_size: self.log_buffer_size,
            dump: self.log_dump,
        };

        (ui_options, scheduler_options, log_options)
    }
}
