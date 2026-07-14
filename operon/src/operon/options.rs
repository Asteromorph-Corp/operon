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
    /// A Postgres-specific convenience over the backend-agnostic
    /// [`from_backend`](Self::from_backend). For finer Postgres tuning, build a
    /// [`PsqlMetaStorageOptions`] and pass it to [`from_backend`](Self::from_backend), which
    /// accepts anything convertible into [`MetaBackendOptions`].
    pub fn from_psql_uri(meta_storage_uri: impl Into<String>) -> Self {
        Self::from_backend(MetaBackendOptions::psql(meta_storage_uri))
    }

    /// Former name of [`from_psql_uri`](Self::from_psql_uri).
    #[deprecated(note = "renamed to `from_psql_uri`")]
    pub fn new(meta_storage_uri: impl Into<String>) -> Self {
        Self::from_psql_uri(meta_storage_uri)
    }

    /// Creates options for an Operon whose metadata lives in the given backend.
    ///
    /// All other settings start at their defaults and can be overridden with the `with_*` builder
    /// methods. Backend-specific tuning (pool size, schema, keepalives for Postgres) lives inside
    /// the [`MetaBackendOptions`] variant itself.
    pub fn from_backend(backend: impl Into<MetaBackendOptions>) -> Self {
        let backend = backend.into();
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

    #[deprecated(
        note = "set Postgres knobs on a PsqlMetaStorageOptions and pass it to `from_backend`"
    )]
    pub fn with_meta_storage_pool_size(self, size: u32) -> Self {
        self.map_psql(|psql| psql.with_pool_size(size))
    }

    #[deprecated(
        note = "set Postgres knobs on a PsqlMetaStorageOptions and pass it to `from_backend`"
    )]
    pub fn with_meta_storage_schema(self, schema: impl Into<String>) -> Self {
        self.map_psql(|psql| psql.with_schema(schema))
    }

    #[deprecated(
        note = "set Postgres knobs on a PsqlMetaStorageOptions and pass it to `from_backend`"
    )]
    pub fn with_meta_storage_keepalives_idle(self, duration: std::time::Duration) -> Self {
        self.map_psql(|psql| psql.with_keepalives_idle(duration))
    }

    #[deprecated(
        note = "set Postgres knobs on a PsqlMetaStorageOptions and pass it to `from_backend`"
    )]
    pub fn with_meta_storage_keepalives_interval(self, duration: std::time::Duration) -> Self {
        self.map_psql(|psql| psql.with_keepalives_interval(duration))
    }

    /// Applies a builder step to the configured backend's [`PsqlMetaStorageOptions`], leaving any
    /// other backend untouched.
    fn map_psql(
        mut self,
        f: impl FnOnce(PsqlMetaStorageOptions) -> PsqlMetaStorageOptions,
    ) -> Self {
        match self.meta_storage_backend {
            MetaBackendOptions::Psql(psql) => {
                self.meta_storage_backend = MetaBackendOptions::Psql(f(psql));
            }
            MetaBackendOptions::Mem(_) => {}
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
