// Exposed interface for the generated Operon architecture.
pub mod operon {
    use super::operon_internal::*;

    // Constants used throughout the library.
    /// The size of the internal channel buffers.
    pub const INTERNAL_CHANNEL_SIZE: usize = 1024;
    /// The URI of the PostgreSQL database used by the metadata storage.
    pub const META_DATABASE_URI: &str = "postgres://crescmoon@localhost/operon-db";
    /// The name of the metadata schema in the PostgreSQL database.
    pub const META_SCHEMA: Option<&'static str> = Some("metadata");
    /// The URI of the PostgreSQL database used by the data storage.
    pub const DATABASE_URI: &str = "postgres://crescmoon@localhost/operon-db";
    /// The name of the data schema in the PostgreSQL database.
    pub const DATA_SCHEMA: Option<&'static str> = Some("data");
    /// The number of logs that the UI keeps in memory.
    pub const LOG_BUFFER_SIZE: usize = 1024;
    /// The minimum log level to display.
    pub const LOG_LEVEL: ::log::Level = ::log::Level::Debug;
    /// Enable or disable log-dumping to a file.
    pub const LOG_DUMP: bool = true;
    pub const LOG_DUMP_DIR: &str = "./logs";

    /// Error type returned by Operon.
    #[derive(Debug)]
    pub enum OperonError {
        /// Error in a storage operation
        Storage(::anyhow::Error),
        /// Error in the scheduler
        Scheduler(::anyhow::Error),
        /// Error in a user function
        User(::anyhow::Error),
        /// Error in the metadata storage
        MetaStorage(::anyhow::Error),
        /// Error in the terminal UI
        UI(::anyhow::Error),
        /// Error caused by missing data
        NotFound(String),
        /// Tried to resolve a ticket with an irrelevant resolution
        InvalidResolution(String),
    }
    impl ::std::fmt::Display for OperonError {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match self {
                OperonError::Storage(e) => write!(f, "Storage error: {e}"),
                OperonError::Scheduler(e) => write!(f, "Scheduler error: {e}"),
                OperonError::User(e) => write!(f, "User function error: {e}"),
                OperonError::MetaStorage(e) => write!(f, "Metadata storage error: {e}"),
                OperonError::UI(e) => write!(f, "Terminal UI error: {e}"),
                OperonError::NotFound(s) => write!(f, "Data expected but not found: {s}"),
                OperonError::InvalidResolution(s) => write!(f, "Invalid resolution: {s}"),
            }
        }
    }
    impl ::std::error::Error for OperonError {}
    pub(super) fn scheduler_error<E: ::std::error::Error + Send + Sync + 'static>(
        e: E,
    ) -> OperonError {
        OperonError::Scheduler(::anyhow::Error::from(e))
    }
    pub(super) fn scheduler_error_str<S: Into<String>>(s: S) -> OperonError {
        OperonError::Scheduler(::anyhow::Error::msg(s.into()))
    }
    pub(super) fn meta_storage_error<E: ::std::error::Error + Send + Sync + 'static>(
        e: E,
    ) -> OperonError {
        OperonError::MetaStorage(::anyhow::Error::from(e))
    }
    pub(super) fn meta_storage_error_str<S: Into<String>>(s: S) -> OperonError {
        OperonError::MetaStorage(::anyhow::Error::msg(s.into()))
    }
    pub(super) fn ui_error<E: ::std::error::Error + Send + Sync + 'static>(e: E) -> OperonError {
        OperonError::UI(::anyhow::Error::from(e))
    }

    /// State of either an `IndividualScheduler` or the whole Operon.
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
    pub(super) enum RunningState {
        Finished,
        #[default]
        Running,
        Paused,
        Error,
        Stopped,
    }
    impl RunningState {
        pub fn color(&self) -> ::ratatui::style::Color {
            match self {
                RunningState::Finished => ::ratatui::style::Color::Green,
                RunningState::Running => ::ratatui::style::Color::Cyan,
                RunningState::Paused => ::ratatui::style::Color::Yellow,
                RunningState::Error => ::ratatui::style::Color::Red,
                RunningState::Stopped => ::ratatui::style::Color::DarkGray,
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, ::clap::ValueEnum)]
    pub(super) enum JobType {
        Beta,
        Gamma,
        Delta,
        Epsilon,
        Zeta,
    }
    impl JobType {
        pub fn is_descendant_of(&self, parent: &JobType) -> bool {
            match self {
                JobType::Beta => matches!(parent, JobType::Beta),
                JobType::Gamma => matches!(parent, JobType::Gamma),
                JobType::Delta => {
                    matches!(parent, JobType::Beta | JobType::Gamma | JobType::Delta)
                }
                JobType::Epsilon => matches!(
                    parent,
                    JobType::Beta | JobType::Gamma | JobType::Delta | JobType::Epsilon
                ),
                JobType::Zeta => matches!(
                    parent,
                    JobType::Beta
                        | JobType::Gamma
                        | JobType::Delta
                        | JobType::Epsilon
                        | JobType::Zeta
                ),
            }
        }
    }
    impl ::std::fmt::Display for JobType {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            match self {
                JobType::Beta => write!(f, "beta"),
                JobType::Gamma => write!(f, "gamma"),
                JobType::Delta => write!(f, "delta"),
                JobType::Epsilon => write!(f, "epsilon"),
                JobType::Zeta => write!(f, "zeta"),
            }
        }
    }

    /// Dimension definitions parsed from the configuration.
    pub mod dimension {
        /// `i`: primary dimension
        pub type I = usize;
        /// `j`: depends on: `i`, derived from: `beta`
        pub type J = usize;
        /// `k`: depends on: `i`, derived from: `gamma`
        pub type K = usize;
    }

    /// Entity definitions parsed from the configuration.
    pub mod entity {
        use serde::{Deserialize, Serialize};
        /// `A`: primary data, repeats on: `i`
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct A(pub String);
        /// `B`: derived from: `beta|i (A)`, repeats on: `i`, `j`
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct B(pub A, pub usize);
        /// `C`: derived from: `gamma|i (A)`, repeats on: `i`, `k`
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct C(pub usize);
        /// `D`: derived from: `delta|i,j,k (A, B, C)`, repeats on: `i`, `j`, `k`
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct D {
            pub a: A,
            pub b: B,
            pub c: C,
        }
        /// `E`: derived from: `epsilon|i,k (B|j, D|j)`, repeats on: `i`, `k`
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct E {
            pub b: Vec<B>,
            pub d: Vec<D>,
        }
        /// `F`: derived from: `zeta|i (C|k, E|k)`, repeats on: `i`
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub enum F {
            Success { c: Vec<C>, e: Vec<E> },
            Failure(String, Option<C>, Option<E>),
        }
    }

    /// # OperonService trait
    ///
    /// This trait contains the functions that are called by Operon.
    /// Implement this trait to provide the operations Operon will run.
    ///
    /// Notes:
    /// * All functions are async methods and must return an `anyhow::Result`.
    /// * Repeated parameters are slices; repeated return types are vectors.
    /// * The functions in the trait are intended to be stateless.
    ///   The struct may contain static metadata, but you would need to implement
    ///   internally-mutable thread-safe states if you really need stateful functions.
    ///
    /// Please consult the following section for exact function signatures.
    ///
    /// ## Function Signatures
    ///
    /// The functions were parsed as follows:
    ///
    /// ```rust
    /// use anyhow::Result;
    /// use async_trait::async_trait;
    /// use operon::entity::*;
    /// #[async_trait]
    /// pub trait OperonService {
    ///     async fn beta(&self, a: &A) -> Result<Vec<B>>;
    ///     async fn gamma(&self, a: &A) -> Result<Vec<C>>;
    ///     async fn delta(&self, a: &A, b: &B, c: &C) -> Result<D>;
    ///     async fn epsilon(&self, b_j: &[B], d_j: &[D]) -> Result<E>;
    ///     async fn zeta(&self, c_k: &[C], e_k: &[E]) -> Result<F>;
    /// }
    /// ```
    #[::async_trait::async_trait]
    pub trait OperonService: Send + Sync + 'static {
        async fn beta(&self, a: &entity::A) -> ::anyhow::Result<Vec<entity::B>>;
        async fn gamma(&self, a: &entity::A) -> ::anyhow::Result<Vec<entity::C>>;
        async fn delta(
            &self,
            a: &entity::A,
            b: &entity::B,
            c: &entity::C,
        ) -> ::anyhow::Result<entity::D>;
        async fn epsilon(
            &self,
            b_j: &[entity::B],
            d_j: &[entity::D],
        ) -> ::anyhow::Result<entity::E>;
        async fn zeta(&self, c_k: &[entity::C], e_k: &[entity::E]) -> ::anyhow::Result<entity::F>;
    }

    /// # OperonStorage trait
    ///
    /// This trait contains the storage operations Operon will use.
    /// Implement this trait to provide a custom storage backend.
    ///
    /// Notes:
    ///
    /// * All functions are async methods and must return an `anyhow::Result`.
    /// * The `clear` function should clear all data EXCEPT the primary data in the storage.
    /// * The `put_*` functions' default behaviour must be to *overwrite* existing data.
    ///   While this is almost never a problem, choosing not to do so may lead to
    ///   undefined behaviour in certain pause-and-resume scenarios.
    /// * The `get_*` functions must return `None` instead of an error if the data is not found.
    /// * The optional `clear_footprint`, `put_footprint` and `get_footprint` functions
    ///   are used to manipulate the footprint of the data.
    ///   The footprint is used to verify the integrity of the data on a recovery from
    ///   previous runs that were gracefully shut down.
    ///   Provide these functions if you want to support fast progress restorations from
    ///   graceful stops.
    /// * Due to having repeated types in the `OperonService` signatures,
    ///   the default implementations of the `put_*` and `get_*` functions
    ///   may cause DB-intensive behaviour.
    ///   If you wish to minimize the number of DB operations,
    ///   you can implement the provided `put_all_*` and `get_all_*_over_*` functions.
    ///   The same rules for the `put_*` and `get_*` functions apply to these as well.
    ///   Additionally, note that these functions assume
    ///   that repeated data is sorted by the dimension it is repeated on.
    ///
    /// Please consult the following section for exact function signatures.
    ///
    /// ## Function Signatures
    ///
    /// The functions were parsed as follows:
    ///
    /// ```rust
    /// use anyhow::Result;
    /// use async_trait::async_trait;
    /// use operon::dimension::*;
    /// use operon::entity::*;
    /// #[async_trait]
    /// pub trait OperonStorage {
    ///     async fn clear(&self) -> Result<()>;
    ///     async fn put_a(&self, i: I, value: &A) -> Result<()>;
    ///     async fn get_a(&self, i: I) -> Result<Option<A>>;
    ///     async fn put_b(&self, i: I, j: J, value: &B) -> Result<()>;
    ///     async fn get_b(&self, i: I, j: J) -> Result<Option<B>>;
    ///     async fn put_c(&self, i: I, k: K, value: &C) -> Result<()>;
    ///     async fn get_c(&self, i: I, k: K) -> Result<Option<C>>;
    ///     async fn put_d(&self, i: I, j: J, k: K, value: &D) -> Result<()>;
    ///     async fn get_d(&self, i: I, j: J, k: K) -> Result<Option<D>>;
    ///     async fn put_e(&self, i: I, k: K, value: &E) -> Result<()>;
    ///     async fn get_e(&self, i: I, k: K) -> Result<Option<E>>;
    ///     async fn put_f(&self, i: I, value: &F) -> Result<()>;
    ///     async fn get_f(&self, i: I) -> Result<Option<F>>;
    ///     // Optional footprint operations:
    ///     async fn clear_footprint(&self) -> Result<()>;
    ///     async fn put_footprint(&self, footprint: &str) -> Result<()>;
    ///     async fn get_footprint(&self) -> Result<Option<String>>;
    ///     // Optional batch operations:
    ///     async fn put_all_b(&self, i: I, values: &[B]) -> Result<()>;
    ///     async fn put_all_c(&self, i: I, values: &[C]) -> Result<()>;
    ///     async fn get_all_b_over_j(&self, i: I) -> Result<Vec<B>>;
    ///     async fn get_all_c_over_k(&self, i: I) -> Result<Vec<C>>;
    ///     async fn get_all_d_over_j(&self, i: I, k: K) -> Result<Vec<D>>;
    ///     async fn get_all_e_over_k(&self, i: I) -> Result<Vec<E>>;
    /// }
    /// ```
    #[async_trait::async_trait]
    pub trait OperonStorage: Send + Sync + 'static {
        async fn clear(&self) -> ::anyhow::Result<()>;

        async fn put_a(&self, i: dimension::I, value: &entity::A) -> ::anyhow::Result<()>;
        async fn get_a(&self, i: dimension::I) -> ::anyhow::Result<Option<entity::A>>;

        async fn put_b(
            &self,
            i: dimension::I,
            j: dimension::J,
            value: &entity::B,
        ) -> ::anyhow::Result<()>;
        async fn get_b(
            &self,
            i: dimension::I,
            j: dimension::J,
        ) -> ::anyhow::Result<Option<entity::B>>;

        async fn put_c(
            &self,
            i: dimension::I,
            k: dimension::K,
            value: &entity::C,
        ) -> ::anyhow::Result<()>;
        async fn get_c(
            &self,
            i: dimension::I,
            k: dimension::K,
        ) -> ::anyhow::Result<Option<entity::C>>;

        async fn put_d(
            &self,
            i: dimension::I,
            j: dimension::J,
            k: dimension::K,
            value: &entity::D,
        ) -> ::anyhow::Result<()>;
        async fn get_d(
            &self,
            i: dimension::I,
            j: dimension::J,
            k: dimension::K,
        ) -> ::anyhow::Result<Option<entity::D>>;

        async fn put_e(
            &self,
            i: dimension::I,
            k: dimension::K,
            value: &entity::E,
        ) -> ::anyhow::Result<()>;
        async fn get_e(
            &self,
            i: dimension::I,
            k: dimension::K,
        ) -> ::anyhow::Result<Option<entity::E>>;

        async fn put_f(&self, i: dimension::I, value: &entity::F) -> ::anyhow::Result<()>;
        async fn get_f(&self, i: dimension::I) -> ::anyhow::Result<Option<entity::F>>;

        async fn clear_footprint(&self) -> ::anyhow::Result<()> {
            // This function is no-op by default, disallowing recovery runs if not implemented.
            Ok(())
        }
        async fn put_footprint(&self, _footprint: &str) -> ::anyhow::Result<()> {
            // This function is no-op by default, disallowing recovery runs if not implemented.
            Ok(())
        }
        async fn get_footprint(&self) -> ::anyhow::Result<Option<String>> {
            // This function is no-op by default, disallowing recovery runs if not implemented.
            Ok(None)
        }

        // Batch operations (only for the operations that are needed in the OperonService trait).
        async fn put_all_b(&self, i: dimension::I, values: &[entity::B]) -> ::anyhow::Result<()> {
            // Default implementation: put each value individually.
            for (j, value) in values.iter().enumerate() {
                self.put_b(i, j, value).await?;
            }
            Ok(())
        }
        async fn put_all_c(&self, i: dimension::I, values: &[entity::C]) -> ::anyhow::Result<()> {
            for (k, value) in values.iter().enumerate() {
                self.put_c(i, k, value).await?;
            }
            Ok(())
        }

        async fn get_all_b_over_j(&self, i: dimension::I) -> ::anyhow::Result<Vec<entity::B>> {
            // Default implementation: get each value individually.
            let mut results = Vec::new();
            for j in 0.. {
                match self.get_b(i, j).await? {
                    Some(value) => results.push(value),
                    None => {
                        // No more values for this `i`.
                        // Note that this might overshoot in specific cases,
                        // like when a previously aborted run left some excess data in the storage,
                        // and the current run only overwrote some of it.
                        // Therefore, it is the job runner's responsibility
                        // to truncate the data coming from here
                        // based on the fact storage resolution.
                        break;
                    }
                }
            }
            Ok(results)
        }
        async fn get_all_c_over_k(&self, i: dimension::I) -> ::anyhow::Result<Vec<entity::C>> {
            let mut results = Vec::new();
            for k in 0.. {
                match self.get_c(i, k).await? {
                    Some(value) => results.push(value),
                    None => break,
                }
            }
            Ok(results)
        }
        async fn get_all_d_over_j(
            &self,
            i: dimension::I,
            k: dimension::K,
        ) -> ::anyhow::Result<Vec<entity::D>> {
            let mut results = Vec::new();
            for j in 0.. {
                match self.get_d(i, j, k).await? {
                    Some(value) => results.push(value),
                    None => break,
                }
            }
            Ok(results)
        }
        async fn get_all_e_over_k(&self, i: dimension::I) -> ::anyhow::Result<Vec<entity::E>> {
            let mut results = Vec::new();
            for k in 0.. {
                match self.get_e(i, k).await? {
                    Some(value) => results.push(value),
                    None => break,
                }
            }
            Ok(results)
        }
    }

    /// # Operon
    ///
    /// The interface for the Operon library.
    ///
    /// Provided a data storage and a service, calling `run` will start executing the jobs.
    pub struct Operon<Sto: OperonStorage, Svc: OperonService> {
        storage: ::std::sync::Arc<Sto>,
        service: ::std::sync::Arc<Svc>,
        ui_state: ::std::sync::Arc<::tokio::sync::RwLock<ui::UiState>>,
    }
    impl<Sto, Svc> Operon<Sto, Svc>
    where
        Sto: OperonStorage,
        Svc: OperonService,
    {
        /// Create a new Operon instance with the given storage and service.
        pub fn new(storage: ::std::sync::Arc<Sto>, service: ::std::sync::Arc<Svc>) -> Self {
            let ui_state =
                ::std::sync::Arc::new(::tokio::sync::RwLock::new(ui::UiState::default()));
            Operon {
                storage,
                service,
                ui_state,
            }
        }
        /// Run the Operon instance with the given primary upper bound.
        ///
        /// This function is intended to be called ONCE in the main thread in a binary executable context.
        /// Running this will take over the terminal, so it is strongly discouraged to make any
        /// other writes to `stdout` or `stderr` while this is running.
        /// Instead, you can use the provided macros to log messages to the UI.
        pub async fn run(&self, primary_upper_bound: usize) -> Result<(), OperonError> {
            // Initialize the logger
            let (log_tx, log_rx) = ::tokio::sync::broadcast::channel(LOG_BUFFER_SIZE);

            // Initialize the control event and recovery state channel
            let (ctrl_tx, ctrl_rx) = ::tokio::sync::watch::channel(scheduler::ControlEvent::Start);
            let (rec_tx, rec_rx) = ::tokio::sync::watch::channel(scheduler::RecoveryState::Unknown);

            // Set up the logger
            let logger = ui::UiLogger::new(log_tx);
            ::log::set_boxed_logger(Box::new(logger)).map_err(ui_error)?;
            ::log::set_max_level(::log::LevelFilter::Trace);

            // Create the scheduler
            let scheduler = scheduler::Scheduler::<Sto, Svc>::new(
                self.storage.clone(),
                self.service.clone(),
                self.ui_state.clone(),
                ctrl_rx,
                rec_tx,
            )
            .await?;

            // Spawn the scheduler thread
            let scheduler_handle = { ::tokio::spawn(async move { scheduler.work().await }) };

            // Main UI loop
            ui::run_ui(
                self.ui_state.clone(),
                primary_upper_bound,
                log_rx,
                ctrl_tx,
                rec_rx,
            )
            .await?;

            // By the time the UI exits, the scheduler should have finished
            scheduler_handle.await.map_err(scheduler_error)??;

            Ok(())
        }
    }

    // Macros exposed to the user.
    pub mod macros {
        /// Log a message at the ERROR level.
        #[macro_export]
        macro_rules! error {
            () => {
                ::log::error!("");
            };
            ($($arg:tt)*) => {
                ::log::error!($($arg)*)
            };
        }
        /// Log a message at the WARN level.
        #[macro_export]
        macro_rules! warn {
            () => {
                ::log::warn!("");
            };
            ($($arg:tt)*) => {
                ::log::warn!($($arg)*)
            };
        }
        /// Log a message at the INFO level.
        #[macro_export]
        macro_rules! info {
            () => {
                ::log::info!("");
            };
            ($($arg:tt)*) => {
                ::log::info!($($arg)*)
            };
        }
        /// Log a message at the DEBUG level.
        #[macro_export]
        macro_rules! debug {
            () => {
                ::log::debug!("");
            };
            ($($arg:tt)*) => {
                ::log::debug!($($arg)*)
            };
        }
        /// Log a message at the TRACE level.
        #[macro_export]
        macro_rules! trace {
            () => {
                ::log::trace!("");
            };
            ($($arg:tt)*) => {
                ::log::trace!($($arg)*)
            };
        }
    }
    // Public re-exports of the macros.
    #[allow(unused_imports)]
    pub use crate::{debug, error, info, trace, warn};
}

mod operon_internal {
    use super::operon::*;
    /// Internal shorthand for a Result with OperonError.
    type Result<T> = ::std::result::Result<T, OperonError>;

    /// Masked dimension definitions.
    ///
    /// These are used to represent the dimensions that are not fully resolved,
    /// especially in the metadata storages and the scheduler.
    pub mod masked_dimension {
        use super::dimension;
        pub trait MaskedDimension {
            fn resolve(&self) -> Option<usize>;
        }
        #[derive(Debug, Clone)]
        pub enum I {
            One(dimension::I),
            All,
        }
        impl MaskedDimension for I {
            fn resolve(&self) -> Option<usize> {
                match self {
                    I::One(i) => Some(*i),
                    I::All => None,
                }
            }
        }
        #[derive(Debug, Clone)]
        pub enum J {
            One(dimension::J),
            All(I),
        }
        impl MaskedDimension for J {
            fn resolve(&self) -> Option<usize> {
                match self {
                    J::One(j) => Some(*j),
                    J::All(_) => None,
                }
            }
        }
        #[derive(Debug, Clone)]
        pub enum K {
            One(dimension::K),
            All(I),
        }
        impl MaskedDimension for K {
            fn resolve(&self) -> Option<usize> {
                match self {
                    K::One(k) => Some(*k),
                    K::All(_) => None,
                }
            }
        }

        /// Dimension resolutions.
        /// First element is the exclusive upper bound of the requested dimension;
        /// the rest denotes the coordinates of the parent dimensions.
        #[derive(Debug, Clone)]
        pub enum Resolution {
            I(dimension::I),
            J(dimension::J, dimension::I),
            K(dimension::K, dimension::I),
        }

        /// Requests to fetch the dimension ranges from the fact storage.
        /// Must include the coordinates of the parent dimensions.
        #[derive(Debug, Clone)]
        #[allow(dead_code)]
        pub enum ResolutionRequest {
            I,
            J(dimension::I),
            K(dimension::I),
        }
    }

    /// Metadata storage operations.
    pub mod meta_storage {
        use super::*;
        use ::futures::SinkExt;
        use ticket::*;

        type ToSql = dyn ::tokio_postgres::types::ToSql + Sync;

        /// Wrapper around `deadpool_postgres::GenericClient` that provides
        /// the `copy_in` method.
        #[::async_trait::async_trait]
        pub trait MetaClient: ::deadpool_postgres::GenericClient {
            async fn copy_in<T, U>(
                &self,
                query: &T,
            ) -> ::std::result::Result<::tokio_postgres::CopyInSink<U>, ::tokio_postgres::Error>
            where
                T: ?Sized + ::tokio_postgres::ToStatement + Send + Sync,
                U: ::bytes::Buf + 'static + Send + Sync;
        }
        #[::async_trait::async_trait]
        impl MetaClient for ::deadpool_postgres::Object {
            async fn copy_in<T, U>(
                &self,
                query: &T,
            ) -> ::std::result::Result<::tokio_postgres::CopyInSink<U>, ::tokio_postgres::Error>
            where
                T: ?Sized + ::tokio_postgres::ToStatement + Send + Sync,
                U: ::bytes::Buf + 'static + Send + Sync,
            {
                ::tokio_postgres::Client::copy_in(self, query).await
            }
        }
        #[::async_trait::async_trait]
        impl MetaClient for ::deadpool_postgres::Transaction<'_> {
            async fn copy_in<T, U>(
                &self,
                query: &T,
            ) -> ::std::result::Result<::tokio_postgres::CopyInSink<U>, ::tokio_postgres::Error>
            where
                T: ?Sized + ::tokio_postgres::ToStatement + Send + Sync,
                U: ::bytes::Buf + 'static + Send + Sync,
            {
                ::tokio_postgres::Transaction::copy_in(self, query).await
            }
        }

        /// Minimal connection information for the metadata storage operations.
        pub struct MetaStorageConnection<'a, Cl>
        where
            Cl: MetaClient + 'a,
        {
            // Both references live as long as this connection.
            pub client: &'a Cl,
            pub schema: &'a Option<String>,
        }
        impl<'a, Cl> Clone for MetaStorageConnection<'a, Cl>
        where
            Cl: MetaClient + 'a,
        {
            fn clone(&self) -> Self {
                *self
            }
        }
        impl<'a, Cl> Copy for MetaStorageConnection<'a, Cl> where Cl: MetaClient + 'a {}
        impl<'a, Cl> MetaStorageConnection<'a, Cl>
        where
            Cl: MetaClient + 'a,
        {
            /// Get the schema prefix.
            pub fn get_prefix(&self) -> String {
                match self.schema {
                    Some(s) => format!("{s}."),
                    None => String::new(),
                }
            }

            pub async fn batch_execute(&self, query: &str) -> Result<()> {
                self.client
                    .batch_execute(query)
                    .await
                    .map_err(meta_storage_error)
            }

            pub async fn execute(&self, query: &str, params: &[&ToSql]) -> Result<u64> {
                self.client
                    .execute(query, params)
                    .await
                    .map_err(meta_storage_error)
            }

            pub async fn query(
                &self,
                query: &str,
                params: &[&ToSql],
            ) -> Result<Vec<::tokio_postgres::Row>> {
                self.client
                    .query(query, params)
                    .await
                    .map_err(meta_storage_error)
            }

            pub async fn query_opt(
                &self,
                query: &str,
                params: &[&ToSql],
            ) -> Result<Option<::tokio_postgres::Row>> {
                self.client
                    .query_opt(query, params)
                    .await
                    .map_err(meta_storage_error)
            }

            pub async fn copy_in<T, U>(&self, query: &T) -> Result<::tokio_postgres::CopyInSink<U>>
            where
                T: ?Sized + ::tokio_postgres::ToStatement + Send + Sync,
                U: ::bytes::Buf + 'static + Send + Sync,
            {
                self.client.copy_in(query).await.map_err(meta_storage_error)
            }
        }

        /// Initialize a connection to the metadata storage.
        pub async fn init_connection<S: Into<String>>(uri: S) -> Result<::deadpool_postgres::Pool> {
            // Might want to make these hardcoded config values configurable.
            let pg_config: ::tokio_postgres::Config = {
                let mut config: ::tokio_postgres::Config =
                    uri.into().parse().map_err(meta_storage_error)?;
                config
                    .keepalives(true)
                    .keepalives_idle(::std::time::Duration::from_secs(60))
                    .keepalives_interval(::std::time::Duration::from_secs(30));
                config
            };
            let manager_config = ::deadpool_postgres::ManagerConfig {
                recycling_method: ::deadpool_postgres::RecyclingMethod::Clean,
            };
            let manager = ::deadpool_postgres::Manager::from_config(
                pg_config,
                ::tokio_postgres::NoTls,
                manager_config,
            );
            ::deadpool_postgres::Pool::builder(manager)
                .max_size(16)
                .build()
                .map_err(meta_storage_error)
        }

        /// If given, initialize the schema in the database.
        pub async fn init_schema<'a, Cl>(conn: MetaStorageConnection<'a, Cl>) -> Result<()>
        where
            Cl: MetaClient + 'a,
        {
            let Some(schema) = conn.schema else {
                return Ok(());
            };
            let create_schema = format!("CREATE SCHEMA IF NOT EXISTS {schema}");
            conn.execute(&create_schema, &[]).await?;
            Ok(())
        }

        /// Operations for the PSQL fact storage.
        ///
        /// Given a connection with an optional schema,
        /// this module provides functions to perform the necessaray operations
        /// on the fact storage.
        pub mod facts_psql {
            use super::*;
            /// Initialize the PSQL fact storage.
            /// Note that this function is idempotent, i.e. calling it multiple times,
            /// or calling it on an already-initialized storage will do nothing.
            ///
            /// This function will `CREATE IF NOT EXISTS` the following tables under the given schema:
            ///
            /// * `dimension_i`:
            ///   ```sql
            ///   CREATE TABLE dimension_i (
            ///       i_ub BIGINT NOT NULL
            ///   )
            ///   ```
            /// * `dimension_j`:
            ///   ```sql
            ///   CREATE TABLE dimension_j (
            ///       i BIGINT,
            ///       j_ub BIGINT NOT NULL,
            ///       PRIMARY KEY (i)
            ///   )
            ///   ```
            /// * `dimension_k`:
            ///   ```sql
            ///   CREATE TABLE dimension_k (
            ///       i BIGINT,
            ///       k_ub BIGINT NOT NULL,
            ///       PRIMARY KEY (i)
            ///   )
            ///   ```
            pub async fn init<'a, Cl>(conn: MetaStorageConnection<'a, Cl>) -> Result<()>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                let create_i_table = format!(
                    "CREATE TABLE IF NOT EXISTS {schema_prefix}dimension_i (
                        i_ub BIGINT NOT NULL
                    );"
                );
                let create_j_table = format!(
                    "CREATE TABLE IF NOT EXISTS {schema_prefix}dimension_j (
                        i BIGINT,
                        j_ub BIGINT NOT NULL,
                        PRIMARY KEY (i)
                    );"
                );
                let create_k_table = format!(
                    "CREATE TABLE IF NOT EXISTS {schema_prefix}dimension_k (
                        i BIGINT,
                        k_ub BIGINT NOT NULL,
                        PRIMARY KEY (i)
                    );"
                );
                conn.execute(&create_i_table, &[]).await?;
                conn.execute(&create_j_table, &[]).await?;
                conn.execute(&create_k_table, &[]).await?;
                Ok(())
            }
            /// Clear the data in the PSQL fact storage, assuming the tables are already initialized.
            pub async fn clear<'a, Cl>(conn: MetaStorageConnection<'a, Cl>) -> Result<()>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                let truncate_i_table = format!("TRUNCATE TABLE {schema_prefix}dimension_i");
                let truncate_j_table = format!("TRUNCATE TABLE {schema_prefix}dimension_j");
                let truncate_k_table = format!("TRUNCATE TABLE {schema_prefix}dimension_k");
                conn.execute(&truncate_i_table, &[]).await?;
                conn.execute(&truncate_j_table, &[]).await?;
                conn.execute(&truncate_k_table, &[]).await?;
                Ok(())
            }

            /// Get the resolution for a given masked dimension request.
            pub async fn get_resolution<'a, Cl>(
                conn: MetaStorageConnection<'a, Cl>,
                request: &masked_dimension::ResolutionRequest,
            ) -> Result<Option<masked_dimension::Resolution>>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                let (stmt, params): (String, Vec<i64>) = match request {
                    masked_dimension::ResolutionRequest::I => (
                        format!("SELECT i_ub FROM {schema_prefix}dimension_i"),
                        vec![],
                    ),
                    masked_dimension::ResolutionRequest::J(i) => (
                        format!("SELECT j_ub FROM {schema_prefix}dimension_j WHERE i = $1"),
                        vec![*i as i64],
                    ),
                    masked_dimension::ResolutionRequest::K(i) => (
                        format!("SELECT k_ub FROM {schema_prefix}dimension_k WHERE i = $1"),
                        vec![*i as i64],
                    ),
                };
                let params: Vec<&ToSql> = params.iter().map(|x| x as &ToSql).collect::<Vec<_>>();
                let row = conn.query_opt(&stmt, &params).await?;
                Ok(row.map(|row| match request {
                    masked_dimension::ResolutionRequest::I => {
                        masked_dimension::Resolution::I(row.get::<_, i64>("i_ub") as usize)
                    }
                    masked_dimension::ResolutionRequest::J(i) => {
                        masked_dimension::Resolution::J(row.get::<_, i64>("j_ub") as usize, *i)
                    }
                    masked_dimension::ResolutionRequest::K(i) => {
                        masked_dimension::Resolution::K(row.get::<_, i64>("k_ub") as usize, *i)
                    }
                }))
            }

            /// Put a resolution into the storage.
            pub async fn put_resolution<'a, Cl>(
                conn: MetaStorageConnection<'a, Cl>,
                resolution: &masked_dimension::Resolution,
            ) -> Result<()>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                match resolution {
                    masked_dimension::Resolution::I(i_ub) => {
                        conn.execute(
                            &format!(
                                "INSERT INTO {schema_prefix}dimension_i (i_ub) VALUES ($1)
                                     ON CONFLICT DO NOTHING"
                            ),
                            &[&(*i_ub as i64)],
                        )
                        .await?;
                    }
                    masked_dimension::Resolution::J(j_ub, i) => {
                        conn.execute(
                            &format!(
                                "INSERT INTO {schema_prefix}dimension_j (i, j_ub) VALUES ($1, $2)
                                     ON CONFLICT DO NOTHING"
                            ),
                            &[&(*i as i64), &(*j_ub as i64)],
                        )
                        .await?;
                    }
                    masked_dimension::Resolution::K(k_ub, i) => {
                        conn.execute(
                            &format!(
                                "INSERT INTO {schema_prefix}dimension_k (i, k_ub) VALUES ($1, $2)
                                     ON CONFLICT DO NOTHING"
                            ),
                            &[&(*i as i64), &(*k_ub as i64)],
                        )
                        .await?;
                    }
                }
                Ok(())
            }
        }

        /// Operations for the PSQL ticket storage.
        /// Given a connection with an optional schema,
        /// this module provides functions to perform the necessary operations
        /// on the ticket storage.
        pub mod tickets_psql {
            #![allow(
                unused_assignments,
                unreachable_patterns,
                clippy::unnecessary_literal_unwrap
            )]

            use super::*;
            /// Initialize the PSQL ticket storage.
            /// Note that this function is idempotent, i.e. calling it multiple times,
            /// or calling it on an already-initialized storage will do nothing.
            ///
            /// This function will `CREATE IF NOT EXISTS` the necessary type and tables under the given schema:
            ///
            /// * `ticket_status`:
            /// ```sql
            /// CREATE TYPE ticket_status AS ENUM (
            ///     'waiting',
            ///     'queued',
            ///     'done',
            /// )
            /// ```
            /// * `ticket_beta`:
            /// ```sql
            /// CREATE TABLE ticket_beta (
            ///     i BIGINT,
            ///     resolved BOOLEAN NOT NULL,
            ///     deps_count BIGINT NOT NULL,
            ///     deps_quota BIGINT,
            ///     deps_done BOOLEAN NOT NULL,
            ///     status ticket_status NOT NULL,
            ///     PRIMARY KEY (i)
            /// )
            /// ```
            /// * `ticket_gamma`:
            /// ```sql
            /// CREATE TABLE ticket_gamma (
            ///     i BIGINT,
            ///     resolved BOOLEAN NOT NULL,
            ///     deps_count BIGINT NOT NULL,
            ///     deps_quota BIGINT,
            ///     deps_done BOOLEAN NOT NULL,
            ///     status ticket_status NOT NULL,
            ///     PRIMARY KEY (i)
            /// )
            /// ```
            /// * `ticket_delta`:
            /// ```sql
            /// CREATE TABLE ticket_delta (
            ///     i BIGINT,
            ///     j BIGINT,
            ///     k BIGINT,
            ///     resolved BOOLEAN NOT NULL,
            ///     deps_count BIGINT NOT NULL,
            ///     deps_quota BIGINT,
            ///     deps_done BOOLEAN NOT NULL,
            ///     status ticket_status NOT NULL,
            ///     PRIMARY KEY (i, j, k)
            /// )
            /// ```
            /// * `ticket_epsilon`:
            /// ```sql
            /// CREATE TABLE ticket_epsilon (
            ///     i BIGINT,
            ///     k BIGINT,
            ///     resolved BOOLEAN NOT NULL,
            ///     deps_count BIGINT NOT NULL,
            ///     deps_quota BIGINT,
            ///     deps_done BOOLEAN NOT NULL,
            ///     status ticket_status NOT NULL,
            ///     PRIMARY KEY (i, k)
            /// )
            /// ```
            /// * `ticket_zeta`:
            /// ```sql
            /// CREATE TABLE ticket_zeta (
            ///     i BIGINT,
            ///     resolved BOOLEAN NOT NULL,
            ///     deps_count BIGINT NOT NULL,
            ///     deps_quota BIGINT,
            ///     deps_done BOOLEAN NOT NULL,
            ///     status ticket_status NOT NULL,
            ///     PRIMARY KEY (i)
            /// )
            /// ```
            ///
            /// Also, each table will have an associated summary table
            /// that keeps track of the number of tickets per status, for example:
            /// * `ticket_beta_status`:
            /// ```sql
            /// CREATE TABLE ticket_beta_status (
            ///     waiting BIGINT NOT NULL,
            ///     queued BIGINT NOT NULL,
            ///     done BIGINT NOT NULL,
            ///     CHECK (
            ///         waiting >= 0 AND
            ///         queued >= 0 AND
            ///         done >= 0
            ///     )
            /// )
            /// ```
            pub async fn init<'a, Cl>(conn: MetaStorageConnection<'a, Cl>) -> Result<()>
            where
                Cl: MetaClient + 'a,
            {
                // Create the type and tables.
                let schema_prefix = conn.get_prefix();
                let type_name = format!("{schema_prefix}ticket_status");
                let create_status_type = format!(
                    "DO $$ BEGIN
                        CREATE TYPE {type_name} AS ENUM (
                            'waiting',
                            'queued',
                            'done'
                        );
                    EXCEPTION
                        WHEN duplicate_object THEN null;
                    END $$;"
                );
                let create_beta_table = format!(
                    "
                    CREATE TABLE IF NOT EXISTS {schema_prefix}ticket_beta (
                        i BIGINT,
                        resolved BOOLEAN NOT NULL,
                        deps_count BIGINT NOT NULL,
                        deps_quota BIGINT,
                        deps_done BOOLEAN NOT NULL,
                        status {type_name} NOT NULL,
                        PRIMARY KEY (i)
                    )"
                );
                let create_gamma_table = format!(
                    "CREATE TABLE IF NOT EXISTS {schema_prefix}ticket_gamma (
                        i BIGINT,
                        resolved BOOLEAN NOT NULL,
                        deps_count BIGINT NOT NULL,
                        deps_quota BIGINT,
                        deps_done BOOLEAN NOT NULL,
                        status {type_name} NOT NULL,
                        PRIMARY KEY (i)
                    )"
                );
                let create_delta_table = format!(
                    "CREATE TABLE IF NOT EXISTS {schema_prefix}ticket_delta (
                        i BIGINT,
                        j BIGINT,
                        k BIGINT,
                        resolved BOOLEAN NOT NULL,
                        deps_count BIGINT NOT NULL,
                        deps_quota BIGINT,
                        deps_done BOOLEAN NOT NULL,
                        status {type_name} NOT NULL,
                        PRIMARY KEY (i, j, k)
                    )"
                );
                let create_epsilon_table = format!(
                    "CREATE TABLE IF NOT EXISTS {schema_prefix}ticket_epsilon (
                        i BIGINT,
                        k BIGINT,
                        resolved BOOLEAN NOT NULL,
                        deps_count BIGINT NOT NULL,
                        deps_quota BIGINT,
                        deps_done BOOLEAN NOT NULL,
                        status {type_name} NOT NULL,
                        PRIMARY KEY (i, k)
                    )"
                );
                let create_zeta_table = format!(
                    "CREATE TABLE IF NOT EXISTS {schema_prefix}ticket_zeta (
                        i BIGINT,
                        resolved BOOLEAN NOT NULL,
                        deps_count BIGINT NOT NULL,
                        deps_quota BIGINT,
                        deps_done BOOLEAN NOT NULL,
                        status {type_name} NOT NULL,
                        PRIMARY KEY (i)
                    )"
                );
                conn.execute(&create_status_type, &[]).await?;
                conn.execute(&create_beta_table, &[]).await?;
                conn.execute(&create_gamma_table, &[]).await?;
                conn.execute(&create_delta_table, &[]).await?;
                conn.execute(&create_epsilon_table, &[]).await?;
                conn.execute(&create_zeta_table, &[]).await?;

                // Create the trigger functions and summary tables,
                // and attach them to the main tables.
                let create_beta_summary = format!(
                    "CREATE TABLE IF NOT EXISTS {schema_prefix}ticket_beta_status (
                        waiting BIGINT NOT NULL DEFAULT 0,
                        queued BIGINT NOT NULL DEFAULT 0,
                        done BIGINT NOT NULL DEFAULT 0,
                        CHECK (
                            waiting >= 0 AND
                            queued >= 0 AND
                            done >= 0
                        )
                    );

                    INSERT INTO {schema_prefix}ticket_beta_status (waiting, queued, done)
                    SELECT 0, 0, 0
                    WHERE NOT EXISTS (
                        SELECT 1 FROM {schema_prefix}ticket_beta_status
                    );

                    CREATE OR REPLACE FUNCTION trg_ticket_beta_status() RETURNS TRIGGER AS $$
                    BEGIN
                        IF TG_OP = 'INSERT' THEN
                            UPDATE {schema_prefix}ticket_beta_status
                            SET
                            waiting = waiting + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'waiting'),
                            queued  = queued  + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'queued'),
                            done    = done    + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'done');

                        ELSIF TG_OP = 'UPDATE' THEN
                            UPDATE {schema_prefix}ticket_beta_status
                            SET
                            waiting = waiting 
                                        - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'waiting')
                                        + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'waiting'),
                            queued  = queued  
                                        - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'queued')
                                        + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'queued'),
                            done    = done    
                                        - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'done')
                                        + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'done');

                        ELSIF TG_OP = 'DELETE' THEN
                            UPDATE {schema_prefix}ticket_beta_status
                            SET
                            waiting = waiting - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'waiting'),
                            queued  = queued  - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'queued'),
                            done    = done    - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'done');
                        END IF;

                        RETURN NULL;
                    END;
                    $$ LANGUAGE plpgsql;

                    CREATE OR REPLACE FUNCTION trg_ticket_beta_truncate() RETURNS TRIGGER AS $$
                    BEGIN
                        UPDATE {schema_prefix}ticket_beta_status
                        SET
                            waiting = 0,
                            queued  = 0,
                            done    = 0;
                        RETURN NULL;
                    END;
                    $$ LANGUAGE plpgsql;

                    CREATE OR REPLACE TRIGGER ticket_beta_status_ins_trg
                        AFTER INSERT ON {schema_prefix}ticket_beta
                        REFERENCING
                            NEW TABLE AS NEW_TABLE
                        FOR EACH STATEMENT
                        EXECUTE FUNCTION trg_ticket_beta_status();
                        
                    CREATE OR REPLACE TRIGGER ticket_beta_status_upd_trg
                        AFTER UPDATE ON {schema_prefix}ticket_beta
                        REFERENCING
                            NEW TABLE AS NEW_TABLE
                            OLD TABLE AS OLD_TABLE
                        FOR EACH STATEMENT
                        EXECUTE FUNCTION trg_ticket_beta_status();
                        
                    CREATE OR REPLACE TRIGGER ticket_beta_status_del_trg
                        AFTER DELETE ON {schema_prefix}ticket_beta
                        REFERENCING
                            OLD TABLE AS OLD_TABLE
                        FOR EACH STATEMENT
                        EXECUTE FUNCTION trg_ticket_beta_status();
                        
                    CREATE OR REPLACE TRIGGER ticket_beta_status_trunc_trg
                        AFTER TRUNCATE ON {schema_prefix}ticket_beta
                        FOR EACH STATEMENT
                        EXECUTE FUNCTION trg_ticket_beta_truncate();"
                );
                let create_gamma_summary = format!(
                    "CREATE TABLE IF NOT EXISTS {schema_prefix}ticket_gamma_status (
                        waiting BIGINT NOT NULL DEFAULT 0,
                        queued BIGINT NOT NULL DEFAULT 0,
                        done BIGINT NOT NULL DEFAULT 0,
                        CHECK (
                            waiting >= 0 AND
                            queued >= 0 AND
                            done >= 0
                        )
                    );

                    INSERT INTO {schema_prefix}ticket_gamma_status (waiting, queued, done)
                    SELECT 0, 0, 0
                    WHERE NOT EXISTS (
                        SELECT 1 FROM {schema_prefix}ticket_gamma_status
                    );

                    CREATE OR REPLACE FUNCTION trg_ticket_gamma_status() RETURNS TRIGGER AS $$
                    BEGIN
                        IF TG_OP = 'INSERT' THEN
                            UPDATE {schema_prefix}ticket_gamma_status
                            SET
                            waiting = waiting + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'waiting'),
                            queued  = queued  + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'queued'),
                            done    = done    + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'done');

                        ELSIF TG_OP = 'UPDATE' THEN
                            UPDATE {schema_prefix}ticket_gamma_status
                            SET
                            waiting = waiting 
                                        - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'waiting')
                                        + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'waiting'),
                            queued  = queued  
                                        - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'queued')
                                        + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'queued'),
                            done    = done    
                                        - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'done')
                                        + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'done');

                        ELSIF TG_OP = 'DELETE' THEN
                            UPDATE {schema_prefix}ticket_gamma_status
                            SET
                            waiting = waiting - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'waiting'),
                            queued  = queued  - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'queued'),
                            done    = done    - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'done');
                        END IF;

                        RETURN NULL;
                    END;
                    $$ LANGUAGE plpgsql;

                    CREATE OR REPLACE FUNCTION trg_ticket_gamma_truncate() RETURNS TRIGGER AS $$
                    BEGIN
                        UPDATE {schema_prefix}ticket_gamma_status
                        SET
                            waiting = 0,
                            queued  = 0,
                            done    = 0;
                        RETURN NULL;
                    END;
                    $$ LANGUAGE plpgsql;

                    CREATE OR REPLACE TRIGGER ticket_gamma_status_ins_trg
                        AFTER INSERT ON {schema_prefix}ticket_gamma
                        REFERENCING
                            NEW TABLE AS NEW_TABLE
                        FOR EACH STATEMENT
                        EXECUTE FUNCTION trg_ticket_gamma_status();
                    
                    CREATE OR REPLACE TRIGGER ticket_gamma_status_upd_trg
                        AFTER UPDATE ON {schema_prefix}ticket_gamma
                        REFERENCING
                            NEW TABLE AS NEW_TABLE
                            OLD TABLE AS OLD_TABLE
                        FOR EACH STATEMENT
                        EXECUTE FUNCTION trg_ticket_gamma_status();

                    CREATE OR REPLACE TRIGGER ticket_gamma_status_del_trg
                        AFTER DELETE ON {schema_prefix}ticket_gamma
                        REFERENCING
                            OLD TABLE AS OLD_TABLE
                        FOR EACH STATEMENT
                        EXECUTE FUNCTION trg_ticket_gamma_status();
                    
                    CREATE OR REPLACE TRIGGER ticket_gamma_status_trunc_trg
                        AFTER TRUNCATE ON {schema_prefix}ticket_gamma
                        FOR EACH STATEMENT
                        EXECUTE FUNCTION trg_ticket_gamma_truncate();"
                );
                let create_delta_summary = format!(
                    "CREATE TABLE IF NOT EXISTS {schema_prefix}ticket_delta_status (
                        waiting BIGINT NOT NULL DEFAULT 0,
                        queued BIGINT NOT NULL DEFAULT 0,
                        done BIGINT NOT NULL DEFAULT 0,
                        CHECK (
                            waiting >= 0 AND
                            queued >= 0 AND
                            done >= 0
                        )
                    );

                    INSERT INTO {schema_prefix}ticket_delta_status (waiting, queued, done)
                    SELECT 0, 0, 0
                    WHERE NOT EXISTS (
                        SELECT 1 FROM {schema_prefix}ticket_delta_status
                    );

                    CREATE OR REPLACE FUNCTION trg_ticket_delta_status() RETURNS TRIGGER AS $$
                    BEGIN
                        IF TG_OP = 'INSERT' THEN
                            UPDATE {schema_prefix}ticket_delta_status
                            SET
                            waiting = waiting + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'waiting'),
                            queued  = queued  + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'queued'),
                            done    = done    + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'done');

                        ELSIF TG_OP = 'UPDATE' THEN
                            UPDATE {schema_prefix}ticket_delta_status
                            SET
                            waiting = waiting 
                                        - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'waiting')
                                        + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'waiting'),
                            queued  = queued  
                                        - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'queued')
                                        + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'queued'),
                            done    = done    
                                        - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'done')
                                        + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'done');

                        ELSIF TG_OP = 'DELETE' THEN
                            UPDATE {schema_prefix}ticket_delta_status
                            SET
                            waiting = waiting - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'waiting'),
                            queued  = queued  - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'queued'),
                            done    = done    - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'done');
                        END IF;

                        RETURN NULL;
                    END;
                    $$ LANGUAGE plpgsql;

                    CREATE OR REPLACE FUNCTION trg_ticket_delta_truncate() RETURNS TRIGGER AS $$
                    BEGIN
                        UPDATE {schema_prefix}ticket_delta_status
                        SET
                            waiting = 0,
                            queued  = 0,
                            done    = 0;
                        RETURN NULL;
                    END;
                    $$ LANGUAGE plpgsql;

                    CREATE OR REPLACE TRIGGER ticket_delta_status_ins_trg
                        AFTER INSERT ON {schema_prefix}ticket_delta
                        REFERENCING
                            NEW TABLE AS NEW_TABLE
                        FOR EACH STATEMENT
                        EXECUTE FUNCTION trg_ticket_delta_status();
                        
                    CREATE OR REPLACE TRIGGER ticket_delta_status_upd_trg
                        AFTER UPDATE ON {schema_prefix}ticket_delta
                        REFERENCING
                            NEW TABLE AS NEW_TABLE
                            OLD TABLE AS OLD_TABLE
                        FOR EACH STATEMENT
                        EXECUTE FUNCTION trg_ticket_delta_status();
                    
                    CREATE OR REPLACE TRIGGER ticket_delta_status_del_trg
                        AFTER DELETE ON {schema_prefix}ticket_delta
                        REFERENCING
                            OLD TABLE AS OLD_TABLE
                        FOR EACH STATEMENT
                        EXECUTE FUNCTION trg_ticket_delta_status();
                        
                    CREATE OR REPLACE TRIGGER ticket_delta_status_trunc_trg
                        AFTER TRUNCATE ON {schema_prefix}ticket_delta
                        FOR EACH STATEMENT
                        EXECUTE FUNCTION trg_ticket_delta_truncate();"
                );
                let create_epsilon_summary = format!(
                    "CREATE TABLE IF NOT EXISTS {schema_prefix}ticket_epsilon_status (
                        waiting BIGINT NOT NULL DEFAULT 0,
                        queued BIGINT NOT NULL DEFAULT 0,
                        done BIGINT NOT NULL DEFAULT 0,
                        CHECK (
                            waiting >= 0 AND
                            queued >= 0 AND
                            done >= 0
                        )
                    );

                    INSERT INTO {schema_prefix}ticket_epsilon_status (waiting, queued, done)
                    SELECT 0, 0, 0
                    WHERE NOT EXISTS (
                        SELECT 1 FROM {schema_prefix}ticket_epsilon_status
                    );

                    CREATE OR REPLACE FUNCTION trg_ticket_epsilon_status() RETURNS TRIGGER AS $$
                    BEGIN
                        IF TG_OP = 'INSERT' THEN
                            UPDATE {schema_prefix}ticket_epsilon_status
                            SET
                            waiting = waiting + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'waiting'),
                            queued  = queued  + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'queued'),
                            done    = done    + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'done');

                        ELSIF TG_OP = 'UPDATE' THEN
                            UPDATE {schema_prefix}ticket_epsilon_status
                            SET
                            waiting = waiting 
                                        - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'waiting')
                                        + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'waiting'),
                            queued  = queued  
                                        - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'queued')
                                        + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'queued'),
                            done    = done    
                                        - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'done')
                                        + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'done');

                        ELSIF TG_OP = 'DELETE' THEN
                            UPDATE {schema_prefix}ticket_epsilon_status
                            SET
                            waiting = waiting - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'waiting'),
                            queued  = queued  - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'queued'),
                            done    = done    - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'done');
                        END IF;

                        RETURN NULL;
                    END;
                    $$ LANGUAGE plpgsql;

                    CREATE OR REPLACE FUNCTION trg_ticket_epsilon_truncate() RETURNS TRIGGER AS $$
                    BEGIN
                        UPDATE {schema_prefix}ticket_epsilon_status
                        SET
                            waiting = 0,
                            queued  = 0,
                            done    = 0;
                        RETURN NULL;
                    END;
                    $$ LANGUAGE plpgsql;

                    CREATE OR REPLACE TRIGGER ticket_epsilon_status_ins_trg
                        AFTER INSERT ON {schema_prefix}ticket_epsilon
                        REFERENCING
                            NEW TABLE AS NEW_TABLE
                        FOR EACH STATEMENT
                        EXECUTE FUNCTION trg_ticket_epsilon_status();
                    
                    CREATE OR REPLACE TRIGGER ticket_epsilon_status_upd_trg
                        AFTER UPDATE ON {schema_prefix}ticket_epsilon
                        REFERENCING
                            NEW TABLE AS NEW_TABLE
                            OLD TABLE AS OLD_TABLE
                        FOR EACH STATEMENT
                        EXECUTE FUNCTION trg_ticket_epsilon_status();

                    CREATE OR REPLACE TRIGGER ticket_epsilon_status_del_trg
                        AFTER DELETE ON {schema_prefix}ticket_epsilon
                        REFERENCING
                            OLD TABLE AS OLD_TABLE
                        FOR EACH STATEMENT
                        EXECUTE FUNCTION trg_ticket_epsilon_status();
                        
                    CREATE OR REPLACE TRIGGER ticket_epsilon_status_trunc_trg
                        AFTER TRUNCATE ON {schema_prefix}ticket_epsilon
                        FOR EACH STATEMENT
                        EXECUTE FUNCTION trg_ticket_epsilon_truncate();"
                );
                let create_zeta_summary = format!(
                    "CREATE TABLE IF NOT EXISTS {schema_prefix}ticket_zeta_status (
                        waiting BIGINT NOT NULL DEFAULT 0,
                        queued BIGINT NOT NULL DEFAULT 0,
                        done BIGINT NOT NULL DEFAULT 0,
                        CHECK (
                            waiting >= 0 AND
                            queued >= 0 AND
                            done >= 0
                        )
                    );

                    INSERT INTO {schema_prefix}ticket_zeta_status (waiting, queued, done)
                    SELECT 0, 0, 0
                    WHERE NOT EXISTS (
                        SELECT 1 FROM {schema_prefix}ticket_zeta_status
                    );

                    CREATE OR REPLACE FUNCTION trg_ticket_zeta_status() RETURNS TRIGGER AS $$
                    BEGIN
                        IF TG_OP = 'INSERT' THEN
                            UPDATE {schema_prefix}ticket_zeta_status
                            SET
                            waiting = waiting + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'waiting'),
                            queued  = queued  + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'queued'),
                            done    = done    + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'done');

                        ELSIF TG_OP = 'UPDATE' THEN
                            UPDATE {schema_prefix}ticket_zeta_status
                            SET
                            waiting = waiting 
                                        - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'waiting')
                                        + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'waiting'),
                            queued  = queued  
                                        - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'queued')
                                        + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'queued'),
                            done    = done    
                                        - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'done')
                                        + (SELECT COUNT(*) FROM NEW_TABLE WHERE status = 'done');

                        ELSIF TG_OP = 'DELETE' THEN
                            UPDATE {schema_prefix}ticket_zeta_status
                            SET
                            waiting = waiting - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'waiting'),
                            queued  = queued  - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'queued'),
                            done    = done    - (SELECT COUNT(*) FROM OLD_TABLE WHERE status = 'done');
                        END IF;

                        RETURN NULL;
                    END;
                    $$ LANGUAGE plpgsql;

                    CREATE OR REPLACE FUNCTION trg_ticket_zeta_truncate() RETURNS TRIGGER AS $$
                    BEGIN
                        UPDATE {schema_prefix}ticket_zeta_status
                        SET
                            waiting = 0,
                            queued  = 0,
                            done    = 0;
                        RETURN NULL;
                    END;
                    $$ LANGUAGE plpgsql;

                    CREATE OR REPLACE TRIGGER ticket_zeta_status_ins_trg
                        AFTER INSERT ON {schema_prefix}ticket_zeta
                        REFERENCING
                            NEW TABLE AS NEW_TABLE
                        FOR EACH STATEMENT
                        EXECUTE FUNCTION trg_ticket_zeta_status();
                    
                    CREATE OR REPLACE TRIGGER ticket_zeta_status_upd_trg
                        AFTER UPDATE ON {schema_prefix}ticket_zeta
                        REFERENCING
                            NEW TABLE AS NEW_TABLE
                            OLD TABLE AS OLD_TABLE
                        FOR EACH STATEMENT
                        EXECUTE FUNCTION trg_ticket_zeta_status();
                    
                    CREATE OR REPLACE TRIGGER ticket_zeta_status_del_trg
                        AFTER DELETE ON {schema_prefix}ticket_zeta
                        REFERENCING
                            OLD TABLE AS OLD_TABLE
                        FOR EACH STATEMENT
                        EXECUTE FUNCTION trg_ticket_zeta_status();
                        
                    CREATE OR REPLACE TRIGGER ticket_zeta_status_trunc_trg
                        AFTER TRUNCATE ON {schema_prefix}ticket_zeta
                        FOR EACH STATEMENT
                        EXECUTE FUNCTION trg_ticket_zeta_truncate();"
                );
                conn.batch_execute(&create_beta_summary).await?;
                conn.batch_execute(&create_gamma_summary).await?;
                conn.batch_execute(&create_delta_summary).await?;
                conn.batch_execute(&create_epsilon_summary).await?;
                conn.batch_execute(&create_zeta_summary).await?;
                Ok(())
            }
            /// Clear the data from the PSQL ticket storage, assuming the tables are already initialized.
            pub async fn clear<'a, Cl>(conn: MetaStorageConnection<'a, Cl>) -> Result<()>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                let truncate_beta = format!("TRUNCATE TABLE {schema_prefix}ticket_beta");
                let truncate_gamma = format!("TRUNCATE TABLE {schema_prefix}ticket_gamma");
                let truncate_delta = format!("TRUNCATE TABLE {schema_prefix}ticket_delta");
                let truncate_epsilon = format!("TRUNCATE TABLE {schema_prefix}ticket_epsilon");
                let truncate_zeta = format!("TRUNCATE TABLE {schema_prefix}ticket_zeta");
                conn.execute(&truncate_beta, &[]).await?;
                conn.execute(&truncate_gamma, &[]).await?;
                conn.execute(&truncate_delta, &[]).await?;
                conn.execute(&truncate_epsilon, &[]).await?;
                conn.execute(&truncate_zeta, &[]).await?;
                Ok(())
            }

            /// Put the default (fully unresolved) tickets into the PSQL ticket storage.
            pub async fn put_default_tickets<'a, Cl>(
                conn: MetaStorageConnection<'a, Cl>,
            ) -> Result<()>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                let beta_stmt = format!(
                    "INSERT INTO {schema_prefix}ticket_beta (i, resolved, deps_count, deps_quota, deps_done, status)
                    VALUES {}
                    ON CONFLICT DO NOTHING",
                    BetaTicket::new().to_sql_insert_params()
                );
                let gamma_stmt = format!(
                    "INSERT INTO {schema_prefix}ticket_gamma (i, resolved, deps_count, deps_quota, deps_done, status)
                    VALUES {}
                    ON CONFLICT DO NOTHING",
                    GammaTicket::new().to_sql_insert_params()
                );
                let delta_stmt = format!(
                    "INSERT INTO {schema_prefix}ticket_delta (i, j, k, resolved, deps_count, deps_quota, deps_done, status)
                    VALUES {}
                    ON CONFLICT DO NOTHING",
                    DeltaTicket::new().to_sql_insert_params()
                );
                let epsilon_stmt = format!(
                    "INSERT INTO {schema_prefix}ticket_epsilon (i, k, resolved, deps_count, deps_quota, deps_done, status)
                    VALUES {}
                    ON CONFLICT DO NOTHING",
                    EpsilonTicket::new().to_sql_insert_params()
                );
                let zeta_stmt = format!(
                    "INSERT INTO {schema_prefix}ticket_zeta (i, resolved, deps_count, deps_quota, deps_done, status)
                    VALUES {}
                    ON CONFLICT DO NOTHING",
                    ZetaTicket::new().to_sql_insert_params()
                );
                conn.execute(&beta_stmt, &[]).await?;
                conn.execute(&gamma_stmt, &[]).await?;
                conn.execute(&delta_stmt, &[]).await?;
                conn.execute(&epsilon_stmt, &[]).await?;
                conn.execute(&zeta_stmt, &[]).await?;
                Ok(())
            }

            /// Raise the dependencies of some beta ticket(s), specified by the masked dimension(s).
            /// Return the tickets that are now ready to run.
            // This function is never used since beta tickets don't have dependencies.
            #[allow(dead_code)]
            pub async fn raise_dep_beta<'a, Cl>(
                conn: MetaStorageConnection<'a, Cl>,
                i: &masked_dimension::I,
            ) -> Result<Vec<BetaTicket>>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                let (stmt, params): (String, Vec<i64>) = {
                    let mut stmt = format!("SELECT * FROM {schema_prefix}ticket_beta");
                    let mut params = Vec::new();
                    if let masked_dimension::I::One(i) = i {
                        params.push(*i as i64);
                        stmt.push_str(format!(" WHERE i = ${}", params.len()).as_str());
                    }
                    stmt = format!(
                        "WITH target AS ({stmt}),
                            popped AS (
                                DELETE FROM {schema_prefix}ticket_beta
                                USING target
                                WHERE ticket_beta.i = target.i
                                RETURNING ticket_beta.*
                            )
                            SELECT * FROM popped"
                    );
                    (stmt, params)
                };
                let params: Vec<&ToSql> = params.iter().map(|x| x as &ToSql).collect::<Vec<_>>();
                let rows = conn.query(&stmt, &params).await?;
                let tickets = rows
                    .into_iter()
                    .map(|row| BetaTicket::from_sql_row(&row))
                    .collect::<Result<Vec<_>>>()?;
                let mut writer = ::bytes::BytesMut::new();
                let ready_tickets = {
                    // We can't write directly into the sink,
                    // because we use the connection inside this loop.
                    // We write to a buffer first.
                    let mut ready_tickets = Vec::new();
                    for ticket in tickets {
                        let mut raised_ticket = ticket.raise_dependency_count(conn).await?;
                        if raised_ticket.is_ready() {
                            raised_ticket.status = TicketStatus::Queued;
                        }
                        writer.extend_from_slice(raised_ticket.to_sql_copy_params().as_bytes());
                        if raised_ticket.is_ready() {
                            ready_tickets.push(raised_ticket);
                        }
                    }
                    ready_tickets
                };
                let stmt = format!(
                    "COPY {schema_prefix}ticket_beta (i, resolved, deps_count, deps_quota, deps_done, status)
                    FROM STDIN WITH (FORMAT csv)"
                );
                let sink: ::tokio_postgres::CopyInSink<::bytes::Bytes> =
                    conn.copy_in(&stmt).await?;
                let mut sink = Box::pin(sink);
                sink.send(writer.freeze())
                    .await
                    .map_err(meta_storage_error)?;
                sink.close().await.map_err(meta_storage_error)?;
                Ok(ready_tickets)
            }

            /// Raise the dependencies of some gamma ticket(s), specified by the masked dimension(s).
            /// Return the tickets that are now ready to run.
            #[allow(dead_code)]
            pub async fn raise_dep_gamma<'a, Cl>(
                conn: MetaStorageConnection<'a, Cl>,
                i: &masked_dimension::I,
            ) -> Result<Vec<GammaTicket>>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                let (stmt, params): (String, Vec<i64>) = {
                    let mut stmt = format!("SELECT * FROM {schema_prefix}ticket_gamma");
                    let mut params = Vec::new();
                    if let masked_dimension::I::One(i) = i {
                        params.push(*i as i64);
                        stmt.push_str(format!(" WHERE i = ${}", params.len()).as_str());
                    }
                    stmt = format!(
                        "WITH target AS ({stmt}),
                            popped AS (
                                DELETE FROM {schema_prefix}ticket_gamma
                                USING target
                                WHERE ticket_gamma.i = target.i
                                RETURNING ticket_gamma.*
                            )
                            SELECT * FROM popped"
                    );
                    (stmt, params)
                };
                let params: Vec<&ToSql> = params.iter().map(|x| x as &ToSql).collect::<Vec<_>>();
                let rows = conn.query(&stmt, &params).await?;
                let tickets = rows
                    .into_iter()
                    .map(|row| GammaTicket::from_sql_row(&row))
                    .collect::<Result<Vec<_>>>()?;
                let mut writer = ::bytes::BytesMut::new();
                let ready_tickets = {
                    let mut ready_tickets = Vec::new();
                    for ticket in tickets {
                        let mut raised_ticket = ticket.raise_dependency_count(conn).await?;
                        if raised_ticket.is_ready() {
                            raised_ticket.status = TicketStatus::Queued;
                        }
                        writer.extend_from_slice(raised_ticket.to_sql_copy_params().as_bytes());
                        if raised_ticket.is_ready() {
                            ready_tickets.push(raised_ticket);
                        }
                    }
                    ready_tickets
                };
                let stmt = format!(
                    "COPY {schema_prefix}ticket_gamma (i, resolved, deps_count, deps_quota, deps_done, status)
                    FROM STDIN WITH (FORMAT csv)"
                );
                let sink: ::tokio_postgres::CopyInSink<::bytes::Bytes> =
                    conn.copy_in(&stmt).await?;
                let mut sink = Box::pin(sink);
                sink.send(writer.freeze())
                    .await
                    .map_err(meta_storage_error)?;
                sink.close().await.map_err(meta_storage_error)?;
                Ok(ready_tickets)
            }

            /// Raise the dependencies of some delta ticket(s), specified by the masked dimension(s).
            /// Return the tickets that are now ready to run.
            pub async fn raise_dep_delta<'a, Cl>(
                conn: MetaStorageConnection<'a, Cl>,
                i: &masked_dimension::I,
                j: &masked_dimension::J,
                k: &masked_dimension::K,
            ) -> Result<Vec<DeltaTicket>>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                let (stmt, params): (String, Vec<i64>) = {
                    let mut stmt = format!("SELECT * FROM {schema_prefix}ticket_delta");
                    let mut params = Vec::new();
                    if let masked_dimension::I::One(i) = i {
                        params.push(*i as i64);
                        stmt.push_str(format!(" WHERE i = ${}", params.len()).as_str());
                    }
                    if let masked_dimension::J::One(j) = j {
                        params.push(*j as i64);
                        stmt.push_str(format!(" AND j = ${}", params.len()).as_str());
                    }
                    if let masked_dimension::K::One(k) = k {
                        params.push(*k as i64);
                        stmt.push_str(format!(" AND k = ${}", params.len()).as_str());
                    }
                    stmt = format!(
                        "WITH target AS ({stmt}),
                            popped AS (
                                DELETE FROM {schema_prefix}ticket_delta
                                USING target
                                WHERE ticket_delta.i = target.i
                                    AND ticket_delta.j = target.j
                                    AND ticket_delta.k = target.k
                                RETURNING ticket_delta.*
                            )
                            SELECT * FROM popped"
                    );
                    (stmt, params)
                };
                let params: Vec<&ToSql> = params.iter().map(|x| x as &ToSql).collect::<Vec<_>>();
                let rows = conn.query(&stmt, &params).await?;
                let tickets = rows
                    .into_iter()
                    .map(|row| DeltaTicket::from_sql_row(&row))
                    .collect::<Result<Vec<_>>>()?;
                let mut writer = ::bytes::BytesMut::new();
                let ready_tickets = {
                    let mut ready_tickets = Vec::new();
                    for ticket in tickets {
                        let mut raised_ticket = ticket.raise_dependency_count(conn).await?;
                        if raised_ticket.is_ready() {
                            raised_ticket.status = TicketStatus::Queued;
                        }
                        writer.extend_from_slice(raised_ticket.to_sql_copy_params().as_bytes());
                        if raised_ticket.is_ready() {
                            ready_tickets.push(raised_ticket);
                        }
                    }
                    ready_tickets
                };
                let stmt = format!(
                    "COPY {schema_prefix}ticket_delta (i, j, k, resolved, deps_count, deps_quota, deps_done, status)
                    FROM STDIN WITH (FORMAT csv)"
                );
                let sink: ::tokio_postgres::CopyInSink<::bytes::Bytes> =
                    conn.copy_in(&stmt).await?;
                let mut sink = Box::pin(sink);
                sink.send(writer.freeze())
                    .await
                    .map_err(meta_storage_error)?;
                sink.close().await.map_err(meta_storage_error)?;
                Ok(ready_tickets)
            }

            /// Raise the dependencies of some epsilon ticket(s), specified by the masked dimension(s).
            /// Return the tickets that are now ready to run.
            pub async fn raise_dep_epsilon<'a, Cl>(
                conn: MetaStorageConnection<'a, Cl>,
                i: &masked_dimension::I,
                k: &masked_dimension::K,
            ) -> Result<Vec<EpsilonTicket>>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                let (stmt, params): (String, Vec<i64>) = {
                    let mut stmt = format!("SELECT * FROM {schema_prefix}ticket_epsilon");
                    let mut params = Vec::new();
                    if let masked_dimension::I::One(i) = i {
                        params.push(*i as i64);
                        stmt.push_str(format!(" WHERE i = ${}", params.len()).as_str());
                    }
                    if let masked_dimension::K::One(k) = k {
                        params.push(*k as i64);
                        stmt.push_str(format!(" AND k = ${}", params.len()).as_str());
                    }
                    stmt = format!(
                        "WITH target AS ({stmt}),
                            popped AS (
                                DELETE FROM {schema_prefix}ticket_epsilon
                                USING target
                                WHERE ticket_epsilon.i = target.i
                                    AND ticket_epsilon.k = target.k
                                RETURNING ticket_epsilon.*
                            )
                            SELECT * FROM popped"
                    );
                    (stmt, params)
                };
                let params: Vec<&ToSql> = params.iter().map(|x| x as &ToSql).collect::<Vec<_>>();
                let rows = conn.query(&stmt, &params).await?;
                let tickets = rows
                    .into_iter()
                    .map(|row| EpsilonTicket::from_sql_row(&row))
                    .collect::<Result<Vec<_>>>()?;
                let mut writer = ::bytes::BytesMut::new();
                let ready_tickets = {
                    let mut ready_tickets = Vec::new();
                    for ticket in tickets {
                        let mut raised_ticket = ticket.raise_dependency_count(conn).await?;
                        if raised_ticket.is_ready() {
                            raised_ticket.status = TicketStatus::Queued;
                        }
                        writer.extend_from_slice(raised_ticket.to_sql_copy_params().as_bytes());
                        if raised_ticket.is_ready() {
                            ready_tickets.push(raised_ticket);
                        }
                    }
                    ready_tickets
                };
                let stmt = format!(
                    "COPY {schema_prefix}ticket_epsilon (i, k, resolved, deps_count, deps_quota, deps_done, status)
                    FROM STDIN WITH (FORMAT csv)"
                );
                let sink: ::tokio_postgres::CopyInSink<::bytes::Bytes> =
                    conn.copy_in(&stmt).await?;
                let mut sink = Box::pin(sink);
                sink.send(writer.freeze())
                    .await
                    .map_err(meta_storage_error)?;
                sink.close().await.map_err(meta_storage_error)?;
                Ok(ready_tickets)
            }

            /// Raise the dependencies of some zeta ticket(s), specified by the masked dimension(s).
            /// Return the tickets that are now ready to run.
            pub async fn raise_dep_zeta<'a, Cl>(
                conn: MetaStorageConnection<'a, Cl>,
                i: &masked_dimension::I,
            ) -> Result<Vec<ZetaTicket>>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                let (stmt, params): (String, Vec<i64>) = {
                    let mut stmt = format!("SELECT * FROM {schema_prefix}ticket_zeta");
                    let mut params = Vec::new();
                    if let masked_dimension::I::One(i) = i {
                        params.push(*i as i64);
                        stmt.push_str(format!(" WHERE i = ${}", params.len()).as_str());
                    }
                    stmt = format!(
                        "WITH target AS ({stmt}),
                            popped AS (
                                DELETE FROM {schema_prefix}ticket_zeta
                                USING target
                                WHERE ticket_zeta.i = target.i
                                RETURNING ticket_zeta.*
                            )
                            SELECT * FROM popped"
                    );
                    (stmt, params)
                };
                let params: Vec<&ToSql> = params.iter().map(|x| x as &ToSql).collect::<Vec<_>>();
                let rows = conn.query(&stmt, &params).await?;
                let tickets = rows
                    .into_iter()
                    .map(|row| ZetaTicket::from_sql_row(&row))
                    .collect::<Result<Vec<_>>>()?;
                let mut writer = ::bytes::BytesMut::new();
                let ready_tickets = {
                    let mut ready_tickets = Vec::new();
                    for ticket in tickets {
                        let mut raised_ticket = ticket.raise_dependency_count(conn).await?;
                        if raised_ticket.is_ready() {
                            raised_ticket.status = TicketStatus::Queued;
                        }
                        writer.extend_from_slice(raised_ticket.to_sql_copy_params().as_bytes());
                        if raised_ticket.is_ready() {
                            ready_tickets.push(raised_ticket);
                        }
                    }
                    ready_tickets
                };
                let stmt = format!(
                    "COPY {schema_prefix}ticket_zeta (i, resolved, deps_count, deps_quota, deps_done, status)
                    FROM STDIN WITH (FORMAT csv)"
                );
                let sink: ::tokio_postgres::CopyInSink<::bytes::Bytes> =
                    conn.copy_in(&stmt).await?;
                let mut sink = Box::pin(sink);
                sink.send(writer.freeze())
                    .await
                    .map_err(meta_storage_error)?;
                sink.close().await.map_err(meta_storage_error)?;
                Ok(ready_tickets)
            }

            /// Given a dimension resolution, apply the resolution to the tickets that can be exploded.
            /// Return the tickets that are now ready to run.
            pub async fn explode_beta<'a, Cl>(
                conn: MetaStorageConnection<'a, Cl>,
                resolution: &masked_dimension::Resolution,
            ) -> Result<Vec<BetaTicket>>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                match resolution {
                    masked_dimension::Resolution::I(i) => {
                        // Statement to select all tickets that match the *parent dimensions* in the resolution.
                        // (In this case, there are none.)
                        let stmt: String = format!(
                            "WITH target AS (SELECT * FROM {schema_prefix}ticket_beta),
                                popped AS (
                                    DELETE FROM {schema_prefix}ticket_beta
                                    USING target
                                    WHERE ticket_beta.i = target.i
                                    RETURNING ticket_beta.*
                                )
                                SELECT * FROM popped"
                        );
                        let params: Vec<i64> = vec![];
                        let params: Vec<&ToSql> =
                            params.iter().map(|x| x as &ToSql).collect::<Vec<_>>();
                        let rows = conn.query(&stmt, &params).await?;
                        let tickets = rows
                            .into_iter()
                            .map(|row| BetaTicket::from_sql_row(&row))
                            .collect::<Result<Vec<_>>>()?;
                        let stmt = format!(
                            "COPY {schema_prefix}ticket_beta (i, resolved, deps_count, deps_quota, deps_done, status)
                            FROM STDIN WITH (FORMAT csv)"
                        );
                        let sink: ::tokio_postgres::CopyInSink<::bytes::Bytes> =
                            conn.copy_in(&stmt).await?;
                        let mut sink = Box::pin(sink);
                        let ready_tickets = {
                            // We can feed to the sink directly,
                            // since we don't need the connection inside the loop
                            // in the explode operation.
                            let mut ready_tickets = Vec::new();
                            for ticket in tickets {
                                let just_exploded =
                                    ticket.explode(masked_dimension::Resolution::I(*i))?;
                                for mut exploded_ticket in just_exploded {
                                    if exploded_ticket.is_ready() {
                                        exploded_ticket.status = TicketStatus::Queued;
                                    }
                                    sink.feed((exploded_ticket.to_sql_copy_params()).into())
                                        .await
                                        .map_err(meta_storage_error)?;
                                    if exploded_ticket.is_ready() {
                                        ready_tickets.push(exploded_ticket);
                                    }
                                }
                            }
                            ready_tickets
                        };
                        sink.close().await.map_err(meta_storage_error)?;
                        Ok(ready_tickets)
                    }
                    _ => Err(OperonError::InvalidResolution(
                        "Called irrelevant explode on beta_i".to_string(),
                    )),
                }
            }

            /// Given a dimension resolution, apply the resolution to the tickets that can be exploded.
            /// Return the tickets that are now ready to run.
            pub async fn explode_gamma<'a, Cl>(
                conn: MetaStorageConnection<'a, Cl>,
                resolution: &masked_dimension::Resolution,
            ) -> Result<Vec<GammaTicket>>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                match resolution {
                    masked_dimension::Resolution::I(i) => {
                        let stmt = format!(
                            "WITH target AS (SELECT * FROM {schema_prefix}ticket_gamma),
                                popped AS (
                                    DELETE FROM {schema_prefix}ticket_gamma
                                    USING target
                                    WHERE ticket_gamma.i = target.i
                                    RETURNING ticket_gamma.*
                                )
                                SELECT * FROM popped"
                        );
                        let params: Vec<i64> = vec![];
                        let params: Vec<&ToSql> =
                            params.iter().map(|x| x as &ToSql).collect::<Vec<_>>();
                        let rows = conn.query(&stmt, &params).await?;
                        let tickets = rows
                            .into_iter()
                            .map(|row| GammaTicket::from_sql_row(&row))
                            .collect::<Result<Vec<_>>>()?;
                        let stmt = format!(
                            "COPY {schema_prefix}ticket_gamma (i, resolved, deps_count, deps_quota, deps_done, status)
                            FROM STDIN WITH (FORMAT csv)"
                        );
                        let sink: ::tokio_postgres::CopyInSink<::bytes::Bytes> =
                            conn.copy_in(&stmt).await?;
                        let mut sink = Box::pin(sink);
                        let ready_tickets = {
                            let mut ready_tickets = Vec::new();
                            for ticket in tickets {
                                let just_exploded =
                                    ticket.explode(masked_dimension::Resolution::I(*i))?;
                                for mut exploded_ticket in just_exploded {
                                    if exploded_ticket.is_ready() {
                                        exploded_ticket.status = TicketStatus::Queued;
                                    }
                                    sink.feed(exploded_ticket.to_sql_copy_params().into())
                                        .await
                                        .map_err(meta_storage_error)?;
                                    if exploded_ticket.is_ready() {
                                        ready_tickets.push(exploded_ticket);
                                    }
                                }
                            }
                            ready_tickets
                        };
                        sink.close().await.map_err(meta_storage_error)?;
                        Ok(ready_tickets)
                    }
                    _ => Err(OperonError::InvalidResolution(
                        "Called irrelevant explode on gamma_i".to_string(),
                    )),
                }
            }

            /// Given a dimension resolution, apply the resolution to the tickets that can be exploded.
            /// Return the tickets that are now ready to run.
            pub async fn explode_delta<'a, Cl>(
                conn: MetaStorageConnection<'a, Cl>,
                resolution: &masked_dimension::Resolution,
            ) -> Result<Vec<DeltaTicket>>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                match resolution {
                    masked_dimension::Resolution::I(i) => {
                        let stmt = format!(
                            "WITH target AS (SELECT * FROM {schema_prefix}ticket_delta),
                                popped AS (
                                    DELETE FROM {schema_prefix}ticket_delta
                                    USING target
                                    WHERE ticket_delta.i = target.i
                                    RETURNING ticket_delta.*
                                )
                                SELECT * FROM popped"
                        );
                        let params: Vec<i64> = vec![];
                        let params: Vec<&ToSql> =
                            params.iter().map(|x| x as &ToSql).collect::<Vec<_>>();
                        let rows = conn.query(&stmt, &params).await?;
                        let tickets = rows
                            .into_iter()
                            .map(|row| DeltaTicket::from_sql_row(&row))
                            .collect::<Result<Vec<_>>>()?;
                        let stmt = format!(
                            "COPY {schema_prefix}ticket_delta (i, j, k, resolved, deps_count, deps_quota, deps_done, status)
                            FROM STDIN WITH (FORMAT csv)"
                        );
                        let sink: ::tokio_postgres::CopyInSink<::bytes::Bytes> =
                            conn.copy_in(&stmt).await?;
                        let mut sink = Box::pin(sink);
                        let ready_tickets = {
                            let mut ready_tickets = Vec::new();
                            for ticket in tickets {
                                let just_exploded =
                                    ticket.explode(masked_dimension::Resolution::I(*i))?;
                                for mut exploded_ticket in just_exploded {
                                    if exploded_ticket.is_ready() {
                                        exploded_ticket.status = TicketStatus::Queued;
                                    }
                                    sink.feed(exploded_ticket.to_sql_copy_params().into())
                                        .await
                                        .map_err(meta_storage_error)?;
                                    if exploded_ticket.is_ready() {
                                        ready_tickets.push(exploded_ticket);
                                    }
                                }
                            }
                            ready_tickets
                        };
                        sink.close().await.map_err(meta_storage_error)?;
                        Ok(ready_tickets)
                    }
                    masked_dimension::Resolution::J(j, i) => {
                        let stmt: String = format!(
                            "WITH target AS (SELECT * FROM {schema_prefix}ticket_delta WHERE i = $1),
                                popped AS (
                                    DELETE FROM {schema_prefix}ticket_delta
                                    USING target
                                    WHERE ticket_delta.i = target.i
                                        AND ticket_delta.j = target.j
                                    RETURNING ticket_delta.*
                                )
                                SELECT * FROM popped"
                        );
                        let params: Vec<i64> = vec![*i as i64];
                        let params: Vec<&ToSql> =
                            params.iter().map(|x| x as &ToSql).collect::<Vec<_>>();
                        let rows = conn.query(&stmt, &params).await?;
                        let tickets = rows
                            .into_iter()
                            .map(|row| DeltaTicket::from_sql_row(&row))
                            .collect::<Result<Vec<_>>>()?;
                        let stmt = format!(
                            "COPY {schema_prefix}ticket_delta (i, j, k, resolved, deps_count, deps_quota, deps_done, status)
                            FROM STDIN WITH (FORMAT csv)"
                        );
                        let sink: ::tokio_postgres::CopyInSink<::bytes::Bytes> =
                            conn.copy_in(&stmt).await?;
                        let mut sink = Box::pin(sink);
                        let ready_tickets = {
                            let mut ready_tickets = Vec::new();
                            for ticket in tickets {
                                let just_exploded =
                                    ticket.explode(masked_dimension::Resolution::J(*j, *i))?;
                                for mut exploded_ticket in just_exploded {
                                    if exploded_ticket.is_ready() {
                                        exploded_ticket.status = TicketStatus::Queued;
                                    }
                                    sink.feed(exploded_ticket.to_sql_copy_params().into())
                                        .await
                                        .map_err(meta_storage_error)?;
                                    if exploded_ticket.is_ready() {
                                        ready_tickets.push(exploded_ticket);
                                    }
                                }
                            }
                            ready_tickets
                        };
                        sink.close().await.map_err(meta_storage_error)?;
                        Ok(ready_tickets)
                    }
                    masked_dimension::Resolution::K(k, i) => {
                        let stmt: String = format!(
                            "WITH target AS (SELECT * FROM {schema_prefix}ticket_delta WHERE i = $1),
                                popped AS (
                                    DELETE FROM {schema_prefix}ticket_delta
                                    USING target
                                    WHERE ticket_delta.i = target.i
                                        AND ticket_delta.k = target.k
                                    RETURNING ticket_delta.*
                                )
                                SELECT * FROM popped"
                        );
                        let params: Vec<i64> = vec![*i as i64];
                        let params: Vec<&ToSql> =
                            params.iter().map(|x| x as &ToSql).collect::<Vec<_>>();
                        let rows = conn.query(&stmt, &params).await?;
                        let tickets = rows
                            .into_iter()
                            .map(|row| DeltaTicket::from_sql_row(&row))
                            .collect::<Result<Vec<_>>>()?;
                        let stmt = format!(
                            "COPY {schema_prefix}ticket_delta (i, j, k, resolved, deps_count, deps_quota, deps_done, status)
                            FROM STDIN WITH (FORMAT csv)"
                        );
                        let sink: ::tokio_postgres::CopyInSink<::bytes::Bytes> =
                            conn.copy_in(&stmt).await?;
                        let mut sink = Box::pin(sink);
                        let ready_tickets = {
                            let mut ready_tickets = Vec::new();
                            for ticket in tickets {
                                let just_exploded =
                                    ticket.explode(masked_dimension::Resolution::K(*k, *i))?;
                                for mut exploded_ticket in just_exploded {
                                    if exploded_ticket.is_ready() {
                                        exploded_ticket.status = TicketStatus::Queued;
                                    }
                                    sink.feed(exploded_ticket.to_sql_copy_params().into())
                                        .await
                                        .map_err(meta_storage_error)?;
                                    if exploded_ticket.is_ready() {
                                        ready_tickets.push(exploded_ticket);
                                    }
                                }
                            }
                            ready_tickets
                        };
                        sink.close().await.map_err(meta_storage_error)?;
                        Ok(ready_tickets)
                    }
                    _ => Err(OperonError::InvalidResolution(
                        "Called irrelevant explode on delta_i,j,k".to_string(),
                    )),
                }
            }

            /// Given a dimension resolution, apply the resolution to the tickets that can be exploded.
            /// Return the tickets that are now ready to run.
            pub async fn explode_epsilon<'a, Cl>(
                conn: MetaStorageConnection<'a, Cl>,
                resolution: &masked_dimension::Resolution,
            ) -> Result<Vec<EpsilonTicket>>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                match resolution {
                    masked_dimension::Resolution::I(i) => {
                        let stmt = format!(
                            "WITH target AS (SELECT * FROM {schema_prefix}ticket_epsilon),
                                popped AS (
                                    DELETE FROM {schema_prefix}ticket_epsilon
                                    USING target
                                    WHERE ticket_epsilon.i = target.i
                                    RETURNING ticket_epsilon.*
                                )
                                SELECT * FROM popped"
                        );
                        let params: Vec<i64> = vec![];
                        let params: Vec<&ToSql> =
                            params.iter().map(|x| x as &ToSql).collect::<Vec<_>>();
                        let rows = conn.query(&stmt, &params).await?;
                        let tickets = rows
                            .into_iter()
                            .map(|row| EpsilonTicket::from_sql_row(&row))
                            .collect::<Result<Vec<_>>>()?;
                        let stmt = format!(
                            "COPY {schema_prefix}ticket_epsilon (i, k, resolved, deps_count, deps_quota, deps_done, status)
                            FROM STDIN WITH (FORMAT csv)"
                        );
                        let sink: ::tokio_postgres::CopyInSink<::bytes::Bytes> =
                            conn.copy_in(&stmt).await?;
                        let mut sink = Box::pin(sink);
                        let ready_tickets = {
                            let mut ready_tickets = Vec::new();
                            for ticket in tickets {
                                let just_exploded =
                                    ticket.explode(masked_dimension::Resolution::I(*i))?;
                                for mut exploded_ticket in just_exploded {
                                    if exploded_ticket.is_ready() {
                                        exploded_ticket.status = TicketStatus::Queued;
                                    }
                                    sink.feed(exploded_ticket.to_sql_copy_params().into())
                                        .await
                                        .map_err(meta_storage_error)?;
                                    if exploded_ticket.is_ready() {
                                        ready_tickets.push(exploded_ticket);
                                    }
                                }
                            }
                            ready_tickets
                        };
                        sink.close().await.map_err(meta_storage_error)?;
                        Ok(ready_tickets)
                    }
                    masked_dimension::Resolution::K(k, i) => {
                        let stmt: String = format!(
                            "WITH target AS (SELECT * FROM {schema_prefix}ticket_epsilon WHERE i = $1),
                                popped AS (
                                    DELETE FROM {schema_prefix}ticket_epsilon
                                    USING target
                                    WHERE ticket_epsilon.i = target.i
                                        AND ticket_epsilon.k = target.k
                                    RETURNING ticket_epsilon.*
                                )
                                SELECT * FROM popped"
                        );
                        let params: Vec<i64> = vec![*i as i64];
                        let params: Vec<&ToSql> =
                            params.iter().map(|x| x as &ToSql).collect::<Vec<_>>();
                        let rows = conn.query(&stmt, &params).await?;
                        let tickets = rows
                            .into_iter()
                            .map(|row| EpsilonTicket::from_sql_row(&row))
                            .collect::<Result<Vec<_>>>()?;
                        let stmt = format!(
                            "COPY {schema_prefix}ticket_epsilon (i, k, resolved, deps_count, deps_quota, deps_done, status)
                            FROM STDIN WITH (FORMAT csv)"
                        );
                        let sink: ::tokio_postgres::CopyInSink<::bytes::Bytes> =
                            conn.copy_in(&stmt).await?;
                        let mut sink = Box::pin(sink);
                        let ready_tickets = {
                            let mut ready_tickets = Vec::new();
                            for ticket in tickets {
                                let just_exploded =
                                    ticket.explode(masked_dimension::Resolution::K(*k, *i))?;
                                for mut exploded_ticket in just_exploded {
                                    if exploded_ticket.is_ready() {
                                        exploded_ticket.status = TicketStatus::Queued;
                                    }
                                    sink.feed(exploded_ticket.to_sql_copy_params().into())
                                        .await
                                        .map_err(meta_storage_error)?;
                                    if exploded_ticket.is_ready() {
                                        ready_tickets.push(exploded_ticket);
                                    }
                                }
                            }
                            ready_tickets
                        };
                        sink.close().await.map_err(meta_storage_error)?;
                        Ok(ready_tickets)
                    }
                    _ => Err(OperonError::InvalidResolution(
                        "Called irrelevant explode on epsilon_i,k".to_string(),
                    )),
                }
            }

            /// Given a dimension resolution, apply the resolution to the tickets that can be exploded.
            /// Return the tickets that are now ready to run.
            pub async fn explode_zeta<'a, Cl>(
                conn: MetaStorageConnection<'a, Cl>,
                resolution: &masked_dimension::Resolution,
            ) -> Result<Vec<ZetaTicket>>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                match resolution {
                    masked_dimension::Resolution::I(i) => {
                        let stmt = format!(
                            "WITH target AS (SELECT * FROM {schema_prefix}ticket_zeta),
                                popped AS (
                                    DELETE FROM {schema_prefix}ticket_zeta
                                    USING target
                                    WHERE ticket_zeta.i = target.i
                                    RETURNING ticket_zeta.*
                                )
                                SELECT * FROM popped"
                        );
                        let params: Vec<i64> = vec![];
                        let params: Vec<&ToSql> =
                            params.iter().map(|x| x as &ToSql).collect::<Vec<_>>();
                        let rows = conn.query(&stmt, &params).await?;
                        let tickets = rows
                            .into_iter()
                            .map(|row| ZetaTicket::from_sql_row(&row))
                            .collect::<Result<Vec<_>>>()?;
                        let stmt = format!(
                            "COPY {schema_prefix}ticket_zeta (i, resolved, deps_count, deps_quota, deps_done, status)
                            FROM STDIN WITH (FORMAT csv)"
                        );
                        let sink: ::tokio_postgres::CopyInSink<::bytes::Bytes> =
                            conn.copy_in(&stmt).await?;
                        let mut sink = Box::pin(sink);
                        let ready_tickets = {
                            let mut ready_tickets = Vec::new();
                            for ticket in tickets {
                                let just_exploded =
                                    ticket.explode(masked_dimension::Resolution::I(*i))?;
                                for mut exploded_ticket in just_exploded {
                                    if exploded_ticket.is_ready() {
                                        exploded_ticket.status = TicketStatus::Queued;
                                    }
                                    sink.feed(exploded_ticket.to_sql_copy_params().into())
                                        .await
                                        .map_err(meta_storage_error)?;
                                    if exploded_ticket.is_ready() {
                                        ready_tickets.push(exploded_ticket);
                                    }
                                }
                            }
                            ready_tickets
                        };
                        sink.close().await.map_err(meta_storage_error)?;
                        Ok(ready_tickets)
                    }
                    _ => Err(OperonError::InvalidResolution(
                        "Called irrelevant explode on zeta_i".to_string(),
                    )),
                }
            }

            /// Mark a beta ticket as done.
            pub async fn mark_done_beta<'a, Cl>(
                conn: MetaStorageConnection<'a, Cl>,
                i: &dimension::I,
            ) -> Result<()>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                let stmt =
                    format!("UPDATE {schema_prefix}ticket_beta SET status = 'done' WHERE i = $1");
                let params: Vec<i64> = vec![*i as i64];
                conn.execute(&stmt, &[&params[0]]).await?;
                Ok(())
            }

            /// Mark a gamma ticket as done.
            pub async fn mark_done_gamma<'a, Cl>(
                conn: MetaStorageConnection<'a, Cl>,
                i: &dimension::I,
            ) -> Result<()>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                let stmt =
                    format!("UPDATE {schema_prefix}ticket_gamma SET status = 'done' WHERE i = $1");
                let params: Vec<i64> = vec![*i as i64];
                conn.execute(&stmt, &[&params[0]]).await?;
                Ok(())
            }

            /// Mark a delta ticket as done.
            pub async fn mark_done_delta<'a, Cl>(
                conn: MetaStorageConnection<'a, Cl>,
                i: &dimension::I,
                j: &dimension::J,
                k: &dimension::K,
            ) -> Result<()>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                let stmt = format!(
                    "UPDATE {schema_prefix}ticket_delta SET status = 'done' WHERE i = $1 AND j = $2 AND k = $3"
                );
                let params: Vec<i64> = vec![*i as i64, *j as i64, *k as i64];
                conn.execute(&stmt, &[&params[0], &params[1], &params[2]])
                    .await?;
                Ok(())
            }

            /// Mark an epsilon ticket as done.
            pub async fn mark_done_epsilon<'a, Cl>(
                conn: MetaStorageConnection<'a, Cl>,
                i: &dimension::I,
                k: &dimension::K,
            ) -> Result<()>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                let stmt = format!(
                    "UPDATE {schema_prefix}ticket_epsilon SET status = 'done' WHERE i = $1 AND k = $2"
                );
                let params: Vec<i64> = vec![*i as i64, *k as i64];
                conn.execute(&stmt, &[&params[0], &params[1]]).await?;
                Ok(())
            }

            /// Mark a zeta ticket as done.
            pub async fn mark_done_zeta<'a, Cl>(
                conn: MetaStorageConnection<'a, Cl>,
                i: &dimension::I,
            ) -> Result<()>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                let stmt =
                    format!("UPDATE {schema_prefix}ticket_zeta SET status = 'done' WHERE i = $1");
                let params: Vec<i64> = vec![*i as i64];
                conn.execute(&stmt, &[&params[0]]).await?;
                Ok(())
            }

            /// Mark a job as done.
            pub async fn mark_done<'a, Cl>(
                conn: MetaStorageConnection<'a, Cl>,
                job: &job::Job,
            ) -> Result<()>
            where
                Cl: MetaClient + 'a,
            {
                match job {
                    job::Job::Beta { i } => {
                        mark_done_beta(conn, i).await?;
                    }
                    job::Job::Gamma { i } => {
                        mark_done_gamma(conn, i).await?;
                    }
                    job::Job::Delta { i, j, k } => {
                        mark_done_delta(conn, i, j, k).await?;
                    }
                    job::Job::Epsilon { i, k } => {
                        mark_done_epsilon(conn, i, k).await?;
                    }
                    job::Job::Zeta { i } => {
                        mark_done_zeta(conn, i).await?;
                    }
                }
                Ok(())
            }

            /// Get the status of the beta tickets.
            pub async fn get_beta_status<'a, Cl>(
                conn: MetaStorageConnection<'a, Cl>,
            ) -> Result<(i64, i64, i64)>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                let stmt = format!("SELECT * FROM {schema_prefix}ticket_beta_status");
                let row = conn
                    .query_opt(&stmt, &[])
                    .await?
                    .ok_or(meta_storage_error_str(
                        "No rows found in ticket_beta_status".to_string(),
                    ))?;
                let done: i64 = row.get("done");
                let queued: i64 = row.get("queued");
                let waiting: i64 = row.get("waiting");
                Ok((done, queued, waiting))
            }

            /// Get the status of the gamma tickets.
            pub async fn get_gamma_status<'a, Cl>(
                conn: MetaStorageConnection<'a, Cl>,
            ) -> Result<(i64, i64, i64)>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                let stmt = format!("SELECT * FROM {schema_prefix}ticket_gamma_status");
                let row = conn
                    .query_opt(&stmt, &[])
                    .await?
                    .ok_or(meta_storage_error_str(
                        "No rows found in ticket_gamma_status".to_string(),
                    ))?;
                let done: i64 = row.get("done");
                let queued: i64 = row.get("queued");
                let waiting: i64 = row.get("waiting");
                Ok((done, queued, waiting))
            }

            /// Get the status of the delta tickets.
            pub async fn get_delta_status<'a, Cl>(
                conn: MetaStorageConnection<'a, Cl>,
            ) -> Result<(i64, i64, i64)>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                let stmt = format!("SELECT * FROM {schema_prefix}ticket_delta_status");
                let row = conn
                    .query_opt(&stmt, &[])
                    .await?
                    .ok_or(meta_storage_error_str(
                        "No rows found in ticket_delta_status".to_string(),
                    ))?;
                let done: i64 = row.get("done");
                let queued: i64 = row.get("queued");
                let waiting: i64 = row.get("waiting");
                Ok((done, queued, waiting))
            }

            /// Get the status of the epsilon tickets.
            pub async fn get_epsilon_status<'a, Cl>(
                conn: MetaStorageConnection<'a, Cl>,
            ) -> Result<(i64, i64, i64)>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                let stmt = format!("SELECT * FROM {schema_prefix}ticket_epsilon_status");
                let row = conn
                    .query_opt(&stmt, &[])
                    .await?
                    .ok_or(meta_storage_error_str(
                        "No rows found in ticket_epsilon_status".to_string(),
                    ))?;
                let done: i64 = row.get("done");
                let queued: i64 = row.get("queued");
                let waiting: i64 = row.get("waiting");
                Ok((done, queued, waiting))
            }

            /// Get the status of the zeta tickets.
            pub async fn get_zeta_status<'a, Cl>(
                conn: MetaStorageConnection<'a, Cl>,
            ) -> Result<(i64, i64, i64)>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                let stmt = format!("SELECT * FROM {schema_prefix}ticket_zeta_status");
                let row = conn
                    .query_opt(&stmt, &[])
                    .await?
                    .ok_or(meta_storage_error_str(
                        "No rows found in ticket_zeta_status".to_string(),
                    ))?;
                let done: i64 = row.get("done");
                let queued: i64 = row.get("queued");
                let waiting: i64 = row.get("waiting");
                Ok((done, queued, waiting))
            }

            /// Get all `done` tickets of a job type.
            pub async fn get_all_done<'a, Cl, T>(
                conn: MetaStorageConnection<'a, Cl>,
            ) -> Result<Vec<T>>
            where
                T: Ticket,
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                let stmt = match T::job_type() {
                    JobType::Beta => {
                        format!("SELECT * FROM {schema_prefix}ticket_beta WHERE status = 'done'")
                    }
                    JobType::Gamma => {
                        format!("SELECT * FROM {schema_prefix}ticket_gamma WHERE status = 'done'")
                    }
                    JobType::Delta => {
                        format!("SELECT * FROM {schema_prefix}ticket_delta WHERE status = 'done'")
                    }
                    JobType::Epsilon => {
                        format!("SELECT * FROM {schema_prefix}ticket_epsilon WHERE status = 'done'")
                    }
                    JobType::Zeta => {
                        format!("SELECT * FROM {schema_prefix}ticket_zeta WHERE status = 'done'")
                    }
                };
                let rows = conn.query(&stmt, &[]).await?;
                let jobs = rows
                    .iter()
                    .map(|row| T::from_sql_row(row))
                    .collect::<Result<Vec<_>>>()?;
                Ok(jobs)
            }

            pub async fn get_all_queued<'a, Cl, T>(
                conn: MetaStorageConnection<'a, Cl>,
            ) -> Result<Vec<T>>
            where
                T: Ticket,
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                let stmt = match T::job_type() {
                    JobType::Beta => {
                        format!("SELECT * FROM {schema_prefix}ticket_beta WHERE status = 'queued'")
                    }
                    JobType::Gamma => {
                        format!("SELECT * FROM {schema_prefix}ticket_gamma WHERE status = 'queued'")
                    }
                    JobType::Delta => {
                        format!("SELECT * FROM {schema_prefix}ticket_delta WHERE status = 'queued'")
                    }
                    JobType::Epsilon => {
                        format!(
                            "SELECT * FROM {schema_prefix}ticket_epsilon WHERE status = 'queued'"
                        )
                    }
                    JobType::Zeta => {
                        format!("SELECT * FROM {schema_prefix}ticket_zeta WHERE status = 'queued'")
                    }
                };
                let rows = conn.query(&stmt, &[]).await?;
                let jobs = rows
                    .iter()
                    .map(|row| T::from_sql_row(row))
                    .collect::<Result<Vec<_>>>()?;
                Ok(jobs)
            }
        }

        /// Operations for footprinting the PSQL metadata storage.
        /// Given a connection with an optional schema,
        /// this module provides operations to footprint the metadata storage.
        pub mod footprint_psql {
            use super::*;
            /// Initialize the footprint table.
            /// Note that this function is idempotent, i.e. calling it multiple times,
            /// or calling it on an already-initialized storage will do nothing.
            pub async fn init<'a, Cl>(conn: MetaStorageConnection<'a, Cl>) -> Result<()>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                let stmt = format!(
                    "CREATE TABLE IF NOT EXISTS {schema_prefix}footprint (
                        key TEXT PRIMARY KEY,
                        value TEXT NOT NULL
                    )"
                );
                conn.execute(&stmt, &[]).await?;
                Ok(())
            }
            /// Clear the footprint table.
            pub async fn clear<'a, Cl>(conn: MetaStorageConnection<'a, Cl>) -> Result<()>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                let stmt = format!("TRUNCATE TABLE {schema_prefix}footprint");
                conn.execute(&stmt, &[]).await?;
                Ok(())
            }
            /// Set a footprint key-value pair.
            pub async fn put_footprint<'a, Cl>(
                conn: MetaStorageConnection<'a, Cl>,
                key: String,
                value: String,
            ) -> Result<()>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                let stmt = format!(
                    "INSERT INTO {schema_prefix}footprint (key, value)
                    VALUES ($1, $2)
                    ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value"
                );
                conn.execute(&stmt, &[&key, &value]).await?;
                Ok(())
            }
            /// Get a footprint value by key.
            pub async fn get_footprint<'a, Cl>(
                conn: MetaStorageConnection<'a, Cl>,
                key: String,
            ) -> Result<Option<String>>
            where
                Cl: MetaClient + 'a,
            {
                let schema_prefix = conn.get_prefix();
                let stmt = format!("SELECT value FROM {schema_prefix}footprint WHERE key = $1");
                let row = conn.query_opt(&stmt, &[&key]).await?;
                Ok(row.map(|r| r.get::<_, &str>(0).to_string()))
            }
        }
    }

    /// Job definitions.
    pub mod job {
        use super::*;
        use meta_storage::*;

        /// Internal jobs that are run by Operon.
        ///
        /// Running these jobs will call the user functions,
        /// fetching necessary data from the storage and storing back the results.
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub enum Job {
            Beta {
                i: dimension::I,
            },
            Gamma {
                i: dimension::I,
            },
            Delta {
                i: dimension::I,
                j: dimension::J,
                k: dimension::K,
            },
            Epsilon {
                i: dimension::I,
                k: dimension::K,
            },
            Zeta {
                i: dimension::I,
            },
        }
        impl Job {
            /// Call the user function and stores the result in the storage.
            ///
            /// Return the dimension resolution that was resolved by this job, if any.
            pub async fn run_job<Sto, Svc, Cl>(
                self,
                conn: MetaStorageConnection<'_, Cl>,
                storage: &Sto,
                service: &Svc,
            ) -> Result<Option<masked_dimension::Resolution>>
            where
                Sto: OperonStorage,
                Svc: OperonService,
                Cl: MetaClient,
            {
                match self {
                    Job::Beta { i } => {
                        let a = storage
                            .get_a(i)
                            .await
                            .map_err(OperonError::Storage)?
                            .ok_or(OperonError::NotFound(format!("A (i={i})")))?;
                        let b_j = service.beta(&a).await.map_err(OperonError::User)?;
                        let j_resolution = b_j.len();
                        storage
                            .put_all_b(i, &b_j)
                            .await
                            .map_err(OperonError::Storage)?;
                        Ok(Some(masked_dimension::Resolution::J(j_resolution, i)))
                    }
                    Job::Gamma { i } => {
                        let a = storage
                            .get_a(i)
                            .await
                            .map_err(OperonError::Storage)?
                            .ok_or(OperonError::NotFound(format!("A (i={i})")))?;
                        let c_k = service.gamma(&a).await.map_err(OperonError::User)?;
                        let k_resolution = c_k.len();
                        storage
                            .put_all_c(i, &c_k)
                            .await
                            .map_err(OperonError::Storage)?;
                        Ok(Some(masked_dimension::Resolution::K(k_resolution, i)))
                    }
                    Job::Delta { i, j, k } => {
                        let a = storage
                            .get_a(i)
                            .await
                            .map_err(OperonError::Storage)?
                            .ok_or(OperonError::NotFound(format!("A (i={i})")))?;
                        let b = storage
                            .get_b(i, j)
                            .await
                            .map_err(OperonError::Storage)?
                            .ok_or(OperonError::NotFound(format!("B (i={i}, j={j})")))?;
                        let c = storage
                            .get_c(i, k)
                            .await
                            .map_err(OperonError::Storage)?
                            .ok_or(OperonError::NotFound(format!("C (i={i}, k={k})")))?;
                        let d = service.delta(&a, &b, &c).await.map_err(OperonError::User)?;
                        storage
                            .put_d(i, j, k, &d)
                            .await
                            .map_err(OperonError::Storage)?;
                        Ok(None)
                    }
                    Job::Epsilon { i, k } => {
                        let j_resolution = facts_psql::get_resolution(
                            conn,
                            &masked_dimension::ResolutionRequest::J(i),
                        )
                        .await?;
                        let j_ub = match j_resolution {
                            Some(masked_dimension::Resolution::J(j_ub, _)) => j_ub,
                            _ => {
                                return Err(OperonError::InvalidResolution(
                                    "Epsilon job requires J resolution".to_string(),
                                ));
                            }
                        };
                        let b_j = {
                            let mut b_j = storage
                                .get_all_b_over_j(i)
                                .await
                                .map_err(OperonError::Storage)?;
                            b_j.truncate(j_ub);
                            if b_j.len() < j_ub {
                                return Err(OperonError::NotFound(format!(
                                    "B (i={i}, j=*) expected {j_ub} elements, found {}",
                                    b_j.len()
                                )));
                            }
                            b_j
                        };
                        let d_j = {
                            let mut d_j = storage
                                .get_all_d_over_j(i, k)
                                .await
                                .map_err(OperonError::Storage)?;
                            d_j.truncate(j_ub);
                            if d_j.len() < j_ub {
                                return Err(OperonError::NotFound(format!(
                                    "D (i={i}, j=*, k={k}) expected {j_ub} elements, found {}",
                                    d_j.len()
                                )));
                            }
                            d_j
                        };
                        let e = service
                            .epsilon(&b_j, &d_j)
                            .await
                            .map_err(OperonError::User)?;
                        storage
                            .put_e(i, k, &e)
                            .await
                            .map_err(OperonError::Storage)?;
                        Ok(None)
                    }
                    Job::Zeta { i } => {
                        let k_resolution = facts_psql::get_resolution(
                            conn,
                            &masked_dimension::ResolutionRequest::K(i),
                        )
                        .await?;
                        let k_ub = match k_resolution {
                            Some(masked_dimension::Resolution::K(k_ub, _)) => k_ub,
                            _ => {
                                return Err(OperonError::InvalidResolution(
                                    "Zeta job requires K resolution".to_string(),
                                ));
                            }
                        };
                        let c_k = {
                            let mut c_k = storage
                                .get_all_c_over_k(i)
                                .await
                                .map_err(OperonError::Storage)?;
                            c_k.truncate(k_ub);
                            if c_k.len() < k_ub {
                                return Err(OperonError::NotFound(format!(
                                    "C (i={i}, k=*) expected {k_ub} elements, found {}",
                                    c_k.len()
                                )));
                            }
                            c_k
                        };
                        let e_k = {
                            let mut e_k = storage
                                .get_all_e_over_k(i)
                                .await
                                .map_err(OperonError::Storage)?;
                            e_k.truncate(k_ub);
                            if e_k.len() < k_ub {
                                return Err(OperonError::NotFound(format!(
                                    "E (i={i}, k=*) expected {k_ub} elements, found {}",
                                    e_k.len()
                                )));
                            }
                            e_k
                        };
                        let f = service.zeta(&c_k, &e_k).await.map_err(OperonError::User)?;
                        storage.put_f(i, &f).await.map_err(OperonError::Storage)?;
                        Ok(None)
                    }
                }
            }
        }
    }

    /// Ticket definitions.
    ///
    /// These are used to represent the jobs that may or may not be fully resolved.
    /// Used in the scheduler to represent the jobs that are waiting to be resolved or run.
    pub mod ticket {
        #![allow(unreachable_patterns)]
        use super::*;
        use masked_dimension::MaskedDimension;
        use meta_storage::*;

        #[derive(Debug, Clone, Copy, PartialEq, Eq, ::postgres_types::FromSql)]
        #[postgres(name = "ticket_status")]
        pub enum TicketStatus {
            #[postgres(name = "waiting")]
            Waiting,
            #[postgres(name = "queued")]
            Queued,
            #[postgres(name = "done")]
            Done,
        }
        impl ::std::fmt::Display for TicketStatus {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                match self {
                    TicketStatus::Waiting => write!(f, "waiting"),
                    TicketStatus::Queued => write!(f, "queued"),
                    TicketStatus::Done => write!(f, "done"),
                }
            }
        }
        impl From<&str> for TicketStatus {
            fn from(s: &str) -> Self {
                match s {
                    "waiting" => TicketStatus::Waiting,
                    "queued" => TicketStatus::Queued,
                    "done" => TicketStatus::Done,
                    _ => panic!("Invalid ticket status: {s}"),
                }
            }
        }

        /// Trait that represents tickets for the jobs.
        #[::async_trait::async_trait]
        pub trait Ticket:
            ::std::fmt::Debug + Default + Clone + Sized + Send + Sync + 'static
        {
            /// Brand-new ticket, with none of the dimensions resolved.
            /// Return the ticket that should be present at startup time.
            fn new() -> Self {
                Self::default()
            }

            /// Fetch the required dependency count for this ticket.
            /// If the quota is not yet known, return `None`.
            ///
            /// This function is async because it may need to access the fact storage
            /// to get the dependency count.
            ///
            /// The dependency quota for each job is computed as the total number of upstream jobs,
            /// where for each type of upstream job, the quota is a sum of products.
            /// As pseudocode:
            ///
            /// ```lua
            /// --[[ "Sink" dimensions are dimensions with no outdegrees,
            ///     i.e. no other relevant dimensions depend on them. ]]
            /// quota = 0
            /// for parent_coordinates in non_sink_dimensions_valid_coordinates do
            ///   product = 1
            ///   for dimension in sink_dimensions do
            ///     -- We call the fact storage to get this upper_bound here
            ///     product *= dimension.upper_bound(parent_coordinates)
            ///   end
            ///   quota += product
            /// end
            /// print(quota)
            /// ```
            async fn get_dependency_quota<Cl>(
                &self,
                conn: MetaStorageConnection<'_, Cl>,
            ) -> Result<Option<usize>>
            where
                Cl: MetaClient;

            /// Raise the dependency count of this ticket by one,
            /// and compute if all dependencies are done.
            /// Return the updated ticket.
            ///
            /// `deps_done` is made true if and only if:
            /// * the dependency quota has become known,
            /// * and the dependency count is equal to the required dependency count.
            async fn raise_dependency_count<Cl>(
                self,
                conn: MetaStorageConnection<'_, Cl>,
            ) -> Result<Self>
            where
                Cl: MetaClient;

            /// Whether this ticket is ready to run,
            /// i.e. whether all dependencies are done and the job is fully resolved.
            fn is_ready(&self) -> bool;

            /// Whether this ticket is fully resolved.
            /// Note that this should go through the masked dimensions,
            /// instead of relying on the precomputed values.
            fn is_resolved(&self) -> bool;

            /// The job that this ticket represents.
            /// If the dimensions or dependencies are not fully resolved, the job will be `None`.
            fn resolve(&self) -> Option<job::Job>;

            /// Given a dimension resolution, consume this ticket and return the updated tickets.
            /// It is an error if the given dimension is irrelevant to this ticket.
            fn explode(self, resolution: masked_dimension::Resolution) -> Result<Vec<Self>>;

            /// Convert into a string that represents the SQL parameters for this ticket,
            /// in the format that can be used in an `INSERT` statement.
            ///
            /// Formatted as "(value,value,'value',\[...\])".
            fn to_sql_insert_params(&self) -> String;

            /// Convert into a CSV string that represents the SQL parameters for this ticket,
            /// in the format that can be used in a `COPY` statement.
            ///
            /// Formatted as "value,value,value,\[...\]\n".
            fn to_sql_copy_params(&self) -> String;

            /// Convert a SQL row into this ticket.
            fn from_sql_row(row: &::tokio_postgres::Row) -> Result<Self>;

            /// The job type for this ticket.
            fn job_type() -> JobType;
        }

        #[derive(Debug, Clone)]
        pub struct BetaTicket {
            i: masked_dimension::I,
            resolved: bool,
            deps_count: usize,
            deps_quota: Option<usize>,
            deps_done: bool,
            pub status: TicketStatus,
        }
        impl Default for BetaTicket {
            fn default() -> Self {
                BetaTicket {
                    i: masked_dimension::I::All,
                    resolved: false,
                    deps_count: 0,
                    deps_quota: Some(0),
                    deps_done: true,
                    status: TicketStatus::Waiting,
                }
            }
        }
        #[::async_trait::async_trait]
        #[automatically_derived]
        impl Ticket for BetaTicket {
            async fn get_dependency_quota<Cl>(
                &self,
                _conn: MetaStorageConnection<'_, Cl>,
            ) -> Result<Option<usize>>
            where
                Cl: MetaClient,
            {
                // beta_i dependencies: none
                debug_assert!(false, "Called get_dependency_quota on beta_i");
                Ok(Some(0))
            }
            async fn raise_dependency_count<Cl>(
                self,
                conn: MetaStorageConnection<'_, Cl>,
            ) -> Result<Self>
            where
                Cl: MetaClient,
            {
                debug_assert!(false, "Called raise_dependency_count on beta_i");
                let mut ticket = self;
                ticket.deps_count += 1;
                if ticket.deps_quota.is_none() {
                    ticket.deps_quota = ticket.get_dependency_quota(conn).await?;
                }
                ticket.deps_done = match ticket.deps_quota {
                    Some(quota) => ticket.deps_count >= quota,
                    None => false,
                };
                Ok(ticket)
            }
            fn is_ready(&self) -> bool {
                self.resolved && self.deps_done
            }
            fn is_resolved(&self) -> bool {
                let mut is_resolved = true;
                is_resolved &= self.i.resolve().is_some();
                is_resolved
            }
            fn resolve(&self) -> Option<job::Job> {
                if self.is_ready() {
                    Some(job::Job::Beta {
                        i: self.i.resolve()?,
                    })
                } else {
                    None
                }
            }
            fn explode(self, resolution: masked_dimension::Resolution) -> Result<Vec<Self>> {
                if self.resolved {
                    return Err(OperonError::InvalidResolution(
                        "Called explode on a fully resolved beta_i".to_string(),
                    ));
                }
                match resolution {
                    masked_dimension::Resolution::I(resolved_i) => match self.i {
                        masked_dimension::I::All => {
                            let mut out_tickets = Vec::new();
                            for i in 0..resolved_i {
                                let mut ticket = self.clone();
                                ticket.i = masked_dimension::I::One(i);
                                ticket.resolved = ticket.is_resolved();
                                out_tickets.push(ticket);
                            }
                            Ok(out_tickets)
                        }
                        _ => Err(OperonError::InvalidResolution(
                            "Called explode(i) on beta, but i was resolved".to_string(),
                        )),
                    },
                    _ => Err(OperonError::InvalidResolution(
                        "Called irrelevant explode on beta_i".to_string(),
                    )),
                }
            }
            fn to_sql_insert_params(&self) -> String {
                format!(
                    "({},{},{},{},{},'{}')",
                    match self.i {
                        masked_dimension::I::All => -1,
                        masked_dimension::I::One(i) => i as i64,
                    },
                    self.resolved,
                    self.deps_count,
                    match self.deps_quota {
                        Some(quota) => quota.to_string(),
                        None => "NULL".to_string(),
                    },
                    self.deps_done,
                    self.status
                )
            }
            fn to_sql_copy_params(&self) -> String {
                format!(
                    "{},{},{},{},{},{}\n",
                    match self.i {
                        masked_dimension::I::All => -1,
                        masked_dimension::I::One(i) => i as i64,
                    },
                    self.resolved,
                    self.deps_count,
                    match self.deps_quota {
                        Some(quota) => quota.to_string(),
                        None => String::new(),
                    },
                    self.deps_done,
                    self.status
                )
            }
            fn from_sql_row(row: &::tokio_postgres::Row) -> Result<Self> {
                let i: i64 = row.get("i");
                let resolved: bool = row.get("resolved");
                let deps_count: i64 = row.get("deps_count");
                let deps_quota: Option<i64> = row.get("deps_quota");
                let deps_done: bool = row.get("deps_done");
                let status: TicketStatus = row.get("status");

                let i = match i {
                    -1 => masked_dimension::I::All,
                    _ => masked_dimension::I::One(i as usize),
                };
                let deps_quota = deps_quota.map(|q| q as usize);
                Ok(BetaTicket {
                    i,
                    resolved,
                    deps_count: deps_count as usize,
                    deps_quota,
                    deps_done,
                    status,
                })
            }
            fn job_type() -> JobType {
                JobType::Beta
            }
        }

        #[derive(Debug, Clone)]
        pub struct GammaTicket {
            i: masked_dimension::I,
            resolved: bool,
            deps_count: usize,
            deps_quota: Option<usize>,
            deps_done: bool,
            pub status: TicketStatus,
        }
        impl Default for GammaTicket {
            fn default() -> Self {
                GammaTicket {
                    i: masked_dimension::I::All,
                    resolved: false,
                    deps_count: 0,
                    deps_quota: Some(0),
                    deps_done: true,
                    status: TicketStatus::Waiting,
                }
            }
        }
        #[::async_trait::async_trait]
        #[automatically_derived]
        impl Ticket for GammaTicket {
            async fn get_dependency_quota<Cl>(
                &self,
                _conn: MetaStorageConnection<'_, Cl>,
            ) -> Result<Option<usize>>
            where
                Cl: MetaClient,
            {
                // gamma_i dependencies: none
                debug_assert!(false, "Called get_dependency_quota on gamma_i");
                Ok(Some(0))
            }
            async fn raise_dependency_count<Cl>(
                self,
                conn: MetaStorageConnection<'_, Cl>,
            ) -> Result<Self>
            where
                Cl: MetaClient,
            {
                debug_assert!(false, "Called raise_dependency_count on gamma_i");
                let mut ticket = self;
                ticket.deps_count += 1;
                if ticket.deps_quota.is_none() {
                    ticket.deps_quota = ticket.get_dependency_quota(conn).await?;
                }
                ticket.deps_done = match ticket.deps_quota {
                    Some(quota) => ticket.deps_count >= quota,
                    None => false,
                };
                Ok(ticket)
            }
            fn is_ready(&self) -> bool {
                self.resolved && self.deps_done
            }
            fn is_resolved(&self) -> bool {
                let mut is_resolved = true;
                is_resolved &= self.i.resolve().is_some();
                is_resolved
            }
            fn resolve(&self) -> Option<job::Job> {
                if self.is_ready() {
                    Some(job::Job::Gamma {
                        i: self.i.resolve()?,
                    })
                } else {
                    None
                }
            }
            fn explode(self, resolution: masked_dimension::Resolution) -> Result<Vec<Self>> {
                if self.resolved {
                    return Err(OperonError::InvalidResolution(
                        "Called explode on a fully resolved gamma_i".to_string(),
                    ));
                }
                match resolution {
                    masked_dimension::Resolution::I(resolved_i) => match self.i {
                        masked_dimension::I::All => {
                            let mut out_tickets = Vec::new();
                            for i in 0..resolved_i {
                                let mut ticket = self.clone();
                                ticket.i = masked_dimension::I::One(i);
                                ticket.resolved = ticket.is_resolved();
                                out_tickets.push(ticket);
                            }
                            Ok(out_tickets)
                        }
                        _ => Err(OperonError::InvalidResolution(
                            "Called explode(i) on gamma, but i was resolved".to_string(),
                        )),
                    },
                    _ => Err(OperonError::InvalidResolution(
                        "Called irrelevant explode on gamma_i".to_string(),
                    )),
                }
            }
            fn to_sql_insert_params(&self) -> String {
                format!(
                    "({},{},{},{},{},'{}')",
                    match self.i {
                        masked_dimension::I::All => -1,
                        masked_dimension::I::One(i) => i as i64,
                    },
                    self.resolved,
                    self.deps_count,
                    match self.deps_quota {
                        Some(quota) => quota.to_string(),
                        None => "NULL".to_string(),
                    },
                    self.deps_done,
                    self.status
                )
            }
            fn to_sql_copy_params(&self) -> String {
                format!(
                    "{},{},{},{},{},{}\n",
                    match self.i {
                        masked_dimension::I::All => -1,
                        masked_dimension::I::One(i) => i as i64,
                    },
                    self.resolved,
                    self.deps_count,
                    match self.deps_quota {
                        Some(quota) => quota.to_string(),
                        None => String::new(),
                    },
                    self.deps_done,
                    self.status
                )
            }
            fn from_sql_row(row: &::tokio_postgres::Row) -> Result<Self> {
                let i: i64 = row.get("i");
                let resolved: bool = row.get("resolved");
                let deps_count: i64 = row.get("deps_count");
                let deps_quota: Option<i64> = row.get("deps_quota");
                let deps_done: bool = row.get("deps_done");
                let status: TicketStatus = row.get("status");

                let i = match i {
                    -1 => masked_dimension::I::All,
                    _ => masked_dimension::I::One(i as usize),
                };
                let deps_quota = deps_quota.map(|q| q as usize);
                Ok(GammaTicket {
                    i,
                    resolved,
                    deps_count: deps_count as usize,
                    deps_quota,
                    deps_done,
                    status,
                })
            }
            fn job_type() -> JobType {
                JobType::Gamma
            }
        }

        #[derive(Debug, Clone)]
        pub struct DeltaTicket {
            i: masked_dimension::I,
            j: masked_dimension::J,
            k: masked_dimension::K,
            resolved: bool,
            deps_count: usize,
            deps_quota: Option<usize>,
            deps_done: bool,
            pub status: TicketStatus,
        }
        impl Default for DeltaTicket {
            fn default() -> Self {
                DeltaTicket {
                    i: masked_dimension::I::All,
                    j: masked_dimension::J::All(masked_dimension::I::All),
                    k: masked_dimension::K::All(masked_dimension::I::All),
                    resolved: false,
                    deps_count: 0,
                    deps_quota: None,
                    deps_done: false,
                    status: TicketStatus::Waiting,
                }
            }
        }
        #[::async_trait::async_trait]
        #[automatically_derived]
        impl Ticket for DeltaTicket {
            async fn get_dependency_quota<Cl>(
                &self,
                _conn: MetaStorageConnection<'_, Cl>,
            ) -> Result<Option<usize>>
            where
                Cl: MetaClient,
            {
                // delta_i,j,k dependencies: beta_i, gamma_i
                Ok(Some(2))
            }
            async fn raise_dependency_count<Cl>(
                self,
                conn: MetaStorageConnection<'_, Cl>,
            ) -> Result<Self>
            where
                Cl: MetaClient,
            {
                let mut ticket = self;
                ticket.deps_count += 1;
                if ticket.deps_quota.is_none() {
                    ticket.deps_quota = ticket.get_dependency_quota(conn).await?;
                }
                ticket.deps_done = match ticket.deps_quota {
                    Some(quota) => ticket.deps_count >= quota,
                    None => false,
                };
                Ok(ticket)
            }
            fn is_ready(&self) -> bool {
                self.resolved && self.deps_done
            }
            fn is_resolved(&self) -> bool {
                let mut is_resolved = true;
                is_resolved &= self.i.resolve().is_some();
                is_resolved &= self.j.resolve().is_some();
                is_resolved &= self.k.resolve().is_some();
                is_resolved
            }
            fn resolve(&self) -> Option<job::Job> {
                if self.is_ready() {
                    Some(job::Job::Delta {
                        i: self.i.resolve()?,
                        j: self.j.resolve()?,
                        k: self.k.resolve()?,
                    })
                } else {
                    None
                }
            }
            fn explode(self, resolution: masked_dimension::Resolution) -> Result<Vec<DeltaTicket>> {
                if self.resolved {
                    return Err(OperonError::InvalidResolution(
                        "Called explode on a fully resolved delta_i,j,k".to_string(),
                    ));
                }
                match resolution {
                    masked_dimension::Resolution::I(resolved_i) => match self.i {
                        masked_dimension::I::All => {
                            let mut out_tickets = Vec::new();
                            for i in 0..resolved_i {
                                let mut ticket = self.clone();
                                ticket.i = masked_dimension::I::One(i);
                                debug_assert!(
                                    ticket.j.resolve().is_none(),
                                    "j(i) was resolved before i"
                                );
                                ticket.j = masked_dimension::J::All(masked_dimension::I::One(i));
                                debug_assert!(
                                    ticket.k.resolve().is_none(),
                                    "k(i) was resolved before i"
                                );
                                ticket.k = masked_dimension::K::All(masked_dimension::I::One(i));
                                ticket.resolved = ticket.is_resolved();
                                out_tickets.push(ticket);
                            }
                            Ok(out_tickets)
                        }
                        _ => Err(OperonError::InvalidResolution(
                            "Called explode(i) on delta, but i was resolved".to_string(),
                        )),
                    },
                    masked_dimension::Resolution::J(resolved_j, resolved_i) => match self.j {
                        masked_dimension::J::All(masked_dimension::I::One(my_i))
                            if resolved_i == my_i =>
                        {
                            let mut out_tickets = Vec::new();
                            for j in 0..resolved_j {
                                let mut ticket = self.clone();
                                ticket.j = masked_dimension::J::One(j);
                                ticket.resolved = ticket.is_resolved();
                                out_tickets.push(ticket);
                            }
                            Ok(out_tickets)
                        }
                        masked_dimension::J::All(_) => Err(OperonError::InvalidResolution(
                            "Resolution of j(i) was given, but i wasn't resolved in this ticket"
                                .to_string(),
                        )),
                        masked_dimension::J::One(_) => Err(OperonError::InvalidResolution(
                            "Called explode(j) on delta, but j was resolved".to_string(),
                        )),
                    },
                    masked_dimension::Resolution::K(resolved_k, resolved_i) => match self.k {
                        masked_dimension::K::All(masked_dimension::I::One(my_i))
                            if resolved_i == my_i =>
                        {
                            let mut out_tickets = Vec::new();
                            for k in 0..resolved_k {
                                let mut ticket = self.clone();
                                ticket.k = masked_dimension::K::One(k);
                                ticket.resolved = ticket.is_resolved();
                                out_tickets.push(ticket);
                            }
                            Ok(out_tickets)
                        }
                        masked_dimension::K::All(_) => Err(OperonError::InvalidResolution(
                            "Resolution of k(i) was given, but i wasn't resolved in this ticket"
                                .to_string(),
                        )),
                        masked_dimension::K::One(_) => Err(OperonError::InvalidResolution(
                            "Called explode(k) on delta, but k was resolved".to_string(),
                        )),
                    },
                    _ => Err(OperonError::InvalidResolution(
                        "Called irrelevant explode on delta_i,j,k".to_string(),
                    )),
                }
            }
            fn to_sql_insert_params(&self) -> String {
                format!(
                    "({},{},{},{},{},{},{},'{}')",
                    match self.i {
                        masked_dimension::I::All => -1,
                        masked_dimension::I::One(i) => i as i64,
                    },
                    match self.j {
                        masked_dimension::J::All(_) => -1,
                        masked_dimension::J::One(j) => j as i64,
                    },
                    match self.k {
                        masked_dimension::K::All(_) => -1,
                        masked_dimension::K::One(k) => k as i64,
                    },
                    self.resolved,
                    self.deps_count,
                    match self.deps_quota {
                        Some(quota) => quota.to_string(),
                        None => "NULL".to_string(),
                    },
                    self.deps_done,
                    self.status
                )
            }
            fn to_sql_copy_params(&self) -> String {
                format!(
                    "{},{},{},{},{},{},{},{}\n",
                    match self.i {
                        masked_dimension::I::All => -1,
                        masked_dimension::I::One(i) => i as i64,
                    },
                    match self.j {
                        masked_dimension::J::All(_) => -1,
                        masked_dimension::J::One(j) => j as i64,
                    },
                    match self.k {
                        masked_dimension::K::All(_) => -1,
                        masked_dimension::K::One(k) => k as i64,
                    },
                    self.resolved,
                    self.deps_count,
                    match self.deps_quota {
                        Some(quota) => quota.to_string(),
                        None => String::new(),
                    },
                    self.deps_done,
                    self.status
                )
            }
            fn from_sql_row(row: &::tokio_postgres::Row) -> Result<Self> {
                let i: i64 = row.get("i");
                let j: i64 = row.get("j");
                let k: i64 = row.get("k");
                let resolved: bool = row.get("resolved");
                let deps_count: i64 = row.get("deps_count");
                let deps_quota: Option<i64> = row.get("deps_quota");
                let deps_done: bool = row.get("deps_done");
                let status: TicketStatus = row.get("status");

                let i = match i {
                    -1 => masked_dimension::I::All,
                    _ => masked_dimension::I::One(i as usize),
                };
                let j = match j {
                    -1 => masked_dimension::J::All(i.clone()),
                    _ => masked_dimension::J::One(j as usize),
                };
                let k = match k {
                    -1 => masked_dimension::K::All(i.clone()),
                    _ => masked_dimension::K::One(k as usize),
                };
                let deps_quota = deps_quota.map(|q| q as usize);
                Ok(DeltaTicket {
                    i,
                    j,
                    k,
                    resolved,
                    deps_count: deps_count as usize,
                    deps_quota,
                    deps_done,
                    status,
                })
            }
            fn job_type() -> JobType {
                JobType::Delta
            }
        }

        #[derive(Debug, Clone)]
        pub struct EpsilonTicket {
            i: masked_dimension::I,
            k: masked_dimension::K,
            resolved: bool,
            deps_count: usize,
            deps_quota: Option<usize>,
            deps_done: bool,
            pub status: TicketStatus,
        }
        impl Default for EpsilonTicket {
            fn default() -> Self {
                EpsilonTicket {
                    i: masked_dimension::I::All,
                    k: masked_dimension::K::All(masked_dimension::I::All),
                    resolved: false,
                    deps_count: 0,
                    deps_quota: None,
                    deps_done: false,
                    status: TicketStatus::Waiting,
                }
            }
        }
        #[::async_trait::async_trait]
        #[automatically_derived]
        impl Ticket for EpsilonTicket {
            async fn get_dependency_quota<Cl>(
                &self,
                conn: MetaStorageConnection<'_, Cl>,
            ) -> Result<Option<usize>>
            where
                Cl: MetaClient,
            {
                // epsilon_i,k dependencies: beta_i, delta_i,j,k
                let Some(i) = self.i.resolve() else {
                    return Ok(None);
                };
                let Some(j_resolution) =
                    facts_psql::get_resolution(conn, &masked_dimension::ResolutionRequest::J(i))
                        .await?
                else {
                    return Ok(None);
                };
                let masked_dimension::Resolution::J(resolved_j, _) = j_resolution else {
                    return Err(OperonError::InvalidResolution(
                        "Epsilon job requires J resolution".to_string(),
                    ));
                };
                Ok(Some(1 + resolved_j))
            }
            async fn raise_dependency_count<Cl>(
                self,
                conn: MetaStorageConnection<'_, Cl>,
            ) -> Result<Self>
            where
                Cl: MetaClient,
            {
                let mut ticket = self;
                ticket.deps_count += 1;
                if ticket.deps_quota.is_none() {
                    ticket.deps_quota = ticket.get_dependency_quota(conn).await?;
                }
                ticket.deps_done = match ticket.deps_quota {
                    Some(quota) => ticket.deps_count >= quota,
                    None => false,
                };
                Ok(ticket)
            }
            fn is_ready(&self) -> bool {
                self.resolved && self.deps_done
            }
            fn is_resolved(&self) -> bool {
                let mut is_resolved = true;
                is_resolved &= self.i.resolve().is_some();
                is_resolved &= self.k.resolve().is_some();
                is_resolved
            }
            fn resolve(&self) -> Option<job::Job> {
                if self.is_ready() {
                    Some(job::Job::Epsilon {
                        i: self.i.resolve()?,
                        k: self.k.resolve()?,
                    })
                } else {
                    None
                }
            }
            fn explode(self, resolution: masked_dimension::Resolution) -> Result<Vec<Self>> {
                if self.resolved {
                    return Err(OperonError::InvalidResolution(
                        "Called explode on a fully resolved epsilon_i,k".to_string(),
                    ));
                }
                match resolution {
                    masked_dimension::Resolution::I(resolved_i) => match self.i.resolve() {
                        None => {
                            let mut out_tickets = Vec::new();
                            for i in 0..resolved_i {
                                let mut ticket = self.clone();
                                ticket.i = masked_dimension::I::One(i);
                                debug_assert!(
                                    ticket.k.resolve().is_none(),
                                    "k(i) was resolved before i"
                                );
                                ticket.k = masked_dimension::K::All(masked_dimension::I::One(i));
                                ticket.resolved = ticket.is_resolved();
                                out_tickets.push(ticket);
                            }
                            Ok(out_tickets)
                        }
                        Some(_) => Err(OperonError::InvalidResolution(
                            "Called explode(i) on epsilon, but i was resolved".to_string(),
                        )),
                    },
                    masked_dimension::Resolution::K(resolved_k, resolved_i) => match self.k {
                        masked_dimension::K::All(masked_dimension::I::One(my_i))
                            if resolved_i == my_i =>
                        {
                            let mut out_tickets = Vec::new();
                            for k in 0..resolved_k {
                                let mut ticket = self.clone();
                                ticket.k = masked_dimension::K::One(k);
                                ticket.resolved = ticket.is_resolved();
                                out_tickets.push(ticket);
                            }
                            Ok(out_tickets)
                        }
                        masked_dimension::K::All(_) => Err(OperonError::InvalidResolution(
                            "Resolution of k(i) was given, but i wasn't resolved in this ticket"
                                .to_string(),
                        )),
                        masked_dimension::K::One(_) => Err(OperonError::InvalidResolution(
                            "Called explode(k) on epsilon, but k was resolved".to_string(),
                        )),
                    },
                    _ => Err(OperonError::InvalidResolution(
                        "Called irrelevant explode on epsilon_i,k".to_string(),
                    )),
                }
            }
            fn to_sql_insert_params(&self) -> String {
                format!(
                    "({},{},{},{},{},{},'{}')",
                    match self.i {
                        masked_dimension::I::All => -1,
                        masked_dimension::I::One(i) => i as i64,
                    },
                    match self.k {
                        masked_dimension::K::All(_) => -1,
                        masked_dimension::K::One(k) => k as i64,
                    },
                    self.resolved,
                    self.deps_count,
                    match self.deps_quota {
                        Some(quota) => quota.to_string(),
                        None => "NULL".to_string(),
                    },
                    self.deps_done,
                    self.status
                )
            }
            fn to_sql_copy_params(&self) -> String {
                format!(
                    "{},{},{},{},{},{},{}\n",
                    match self.i {
                        masked_dimension::I::All => -1,
                        masked_dimension::I::One(i) => i as i64,
                    },
                    match self.k {
                        masked_dimension::K::All(_) => -1,
                        masked_dimension::K::One(k) => k as i64,
                    },
                    self.resolved,
                    self.deps_count,
                    match self.deps_quota {
                        Some(quota) => quota.to_string(),
                        None => String::new(),
                    },
                    self.deps_done,
                    self.status
                )
            }
            fn from_sql_row(row: &::tokio_postgres::Row) -> Result<Self> {
                let i: i64 = row.get("i");
                let k: i64 = row.get("k");
                let resolved: bool = row.get("resolved");
                let deps_count: i64 = row.get("deps_count");
                let deps_quota: Option<i64> = row.get("deps_quota");
                let deps_done: bool = row.get("deps_done");
                let status: TicketStatus = row.get("status");

                let i = match i {
                    -1 => masked_dimension::I::All,
                    _ => masked_dimension::I::One(i as usize),
                };
                let k = match k {
                    -1 => masked_dimension::K::All(i.clone()),
                    _ => masked_dimension::K::One(k as usize),
                };
                let deps_quota = deps_quota.map(|q| q as usize);
                Ok(EpsilonTicket {
                    i,
                    k,
                    resolved,
                    deps_count: deps_count as usize,
                    deps_quota,
                    deps_done,
                    status,
                })
            }
            fn job_type() -> JobType {
                JobType::Epsilon
            }
        }

        #[derive(Debug, Clone)]
        pub struct ZetaTicket {
            i: masked_dimension::I,
            resolved: bool,
            deps_count: usize,
            deps_quota: Option<usize>,
            deps_done: bool,
            pub status: TicketStatus,
        }
        impl Default for ZetaTicket {
            fn default() -> Self {
                ZetaTicket {
                    i: masked_dimension::I::All,
                    resolved: false,
                    deps_count: 0,
                    deps_quota: None,
                    deps_done: false,
                    status: TicketStatus::Waiting,
                }
            }
        }
        #[::async_trait::async_trait]
        #[automatically_derived]
        impl Ticket for ZetaTicket {
            async fn get_dependency_quota<Cl>(
                &self,
                conn: MetaStorageConnection<'_, Cl>,
            ) -> Result<Option<usize>>
            where
                Cl: MetaClient,
            {
                // zeta_i dependencies: gamma_i, epsilon_i,k
                let Some(i) = self.i.resolve() else {
                    return Ok(None);
                };
                let Some(k_resolution) =
                    facts_psql::get_resolution(conn, &masked_dimension::ResolutionRequest::K(i))
                        .await?
                else {
                    return Ok(None);
                };
                let masked_dimension::Resolution::K(resolved_k, _) = k_resolution else {
                    return Err(OperonError::InvalidResolution(
                        "Zeta job requires K resolution".to_string(),
                    ));
                };
                Ok(Some(1 + resolved_k))
            }
            async fn raise_dependency_count<Cl>(
                self,
                conn: MetaStorageConnection<'_, Cl>,
            ) -> Result<Self>
            where
                Cl: MetaClient,
            {
                let mut ticket = self;
                ticket.deps_count += 1;
                if ticket.deps_quota.is_none() {
                    ticket.deps_quota = ticket.get_dependency_quota(conn).await?;
                }
                ticket.deps_done = match ticket.deps_quota {
                    Some(quota) => ticket.deps_count >= quota,
                    None => false,
                };
                Ok(ticket)
            }
            fn is_ready(&self) -> bool {
                self.resolved && self.deps_done
            }
            fn is_resolved(&self) -> bool {
                let mut is_resolved = true;
                is_resolved &= self.i.resolve().is_some();
                is_resolved
            }
            fn resolve(&self) -> Option<job::Job> {
                if self.is_ready() {
                    Some(job::Job::Zeta {
                        i: self.i.resolve()?,
                    })
                } else {
                    None
                }
            }
            fn explode(self, resolution: masked_dimension::Resolution) -> Result<Vec<Self>> {
                if self.resolved {
                    return Err(OperonError::InvalidResolution(
                        "Called explode on a fully resolved zeta_i".to_string(),
                    ));
                }
                match resolution {
                    masked_dimension::Resolution::I(resolved_i) => match self.i.resolve() {
                        None => {
                            let mut out_tickets = Vec::new();
                            for i in 0..resolved_i {
                                let mut ticket = self.clone();
                                ticket.i = masked_dimension::I::One(i);
                                ticket.resolved = ticket.is_resolved();
                                out_tickets.push(ticket);
                            }
                            Ok(out_tickets)
                        }
                        Some(_) => Err(OperonError::InvalidResolution(
                            "Called explode(i) on zeta, but i was resolved".to_string(),
                        )),
                    },
                    _ => Err(OperonError::InvalidResolution(
                        "Called irrelevant explode on zeta_i".to_string(),
                    )),
                }
            }
            fn to_sql_insert_params(&self) -> String {
                format!(
                    "({},{},{},{},{},'{}')",
                    match self.i {
                        masked_dimension::I::All => -1,
                        masked_dimension::I::One(i) => i as i64,
                    },
                    self.resolved,
                    self.deps_count,
                    match self.deps_quota {
                        Some(quota) => quota.to_string(),
                        None => "NULL".to_string(),
                    },
                    self.deps_done,
                    self.status
                )
            }
            fn to_sql_copy_params(&self) -> String {
                format!(
                    "{},{},{},{},{},{}\n",
                    match self.i {
                        masked_dimension::I::All => -1,
                        masked_dimension::I::One(i) => i as i64,
                    },
                    self.resolved,
                    self.deps_count,
                    match self.deps_quota {
                        Some(quota) => quota.to_string(),
                        None => String::new(),
                    },
                    self.deps_done,
                    self.status
                )
            }
            fn from_sql_row(row: &::tokio_postgres::Row) -> Result<Self> {
                let i: i64 = row.get("i");
                let resolved: bool = row.get("resolved");
                let deps_count: i64 = row.get("deps_count");
                let deps_quota: Option<i64> = row.get("deps_quota");
                let deps_done: bool = row.get("deps_done");
                let status: TicketStatus = row.get("status");

                let i = match i {
                    -1 => masked_dimension::I::All,
                    _ => masked_dimension::I::One(i as usize),
                };
                let deps_quota = deps_quota.map(|q| q as usize);
                Ok(ZetaTicket {
                    i,
                    resolved,
                    deps_count: deps_count as usize,
                    deps_quota,
                    deps_done,
                    status,
                })
            }
            fn job_type() -> JobType {
                JobType::Zeta
            }
        }
    }

    /// Scheduler definitions.
    pub mod scheduler {
        #![allow(unused_variables, unreachable_patterns, clippy::match_single_binding)]

        use super::*;
        use meta_storage::*;
        use ticket::*;

        /// `IndividualScheduler`-`IndividualScheduler` communication events.
        ///
        /// These are used for communication between individual schedulers,
        /// where each scheduler should modify its tickets based on the events.
        #[derive(Debug, Clone)]
        enum PeerEvent {
            /// A job was run and finished.
            Job(job::Job),
            /// A resolution was made known.
            Resolution(masked_dimension::Resolution),
        }

        /// `IndividualScheduler`-worker communication events.
        ///
        /// These are used for communication between individual schedulers and their
        /// child worker coroutines.
        #[derive(Debug)]
        enum InternalEvent {
            /// A job successfully finished.
            JobSuccess(job::Job, Option<masked_dimension::Resolution>),
            /// A job failed.
            JobFailure(job::Job, OperonError),
        }

        /// `IndividualScheduler`-UI communication events.
        ///
        /// These are used for communication between individual schedulers and the UI.
        #[derive(Default, Debug, Clone, PartialEq, Eq)]
        pub enum ControlEvent {
            /// Start the scheduler, finding out the recovery state.
            #[default]
            Start,
            /// Perform a check on the consistency between the storages.
            Check { primary_ub: usize },
            /// Perform a clean run.
            CleanRun { primary_ub: usize },
            /// Perform a rebuilding run from an `AbortedChecked` state.
            RebuildRun { primary_ub: usize },
            /// Perform a restoring run from a `GracefullyStopped` state.
            RestoreRun { primary_ub: usize },
            /// Pause executing new jobs.
            /// Note that pausing the scheduler does not stop ongoing jobs,
            /// neither does it stop handling events (i.e. updating the ticket storage).
            Pause {
                targets: Vec<JobType>,
                cascade: bool,
            },
            /// Resume executing new jobs.
            Resume { targets: Vec<JobType> },
            /// Gracefully stop the scheduler:
            /// * Stop executing new jobs (= pause),
            /// * wait for all ongoing jobs to finish,
            /// * handle all pending events,
            /// * place a persistent indication that the scheduler stopped gracefully,
            /// * and then exit.
            ///
            /// Effectively, "pause everything and wait long enough".
            GracefulStop,
            /// Abort the scheduler immediately.
            Abort,
        }

        enum RunMode {
            Clean,
            Rebuild,
            Restore,
        }

        /// Information about how recoverable the last run was.
        /// Recovery methods that can be used to recover lower variants
        /// can also be used to recover higher variants.
        ///
        /// Generally, this means that the higher the variant,
        /// the more information is available/trustworthy.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[repr(u8)]
        pub enum RecoveryState {
            /// Not yet known.
            Unknown,
            /// The current scheduler made an error.
            Error,
            /// Cannot be recovered at all.
            /// This is the case where the metadata storage is empty (e.g. the scheduler was never run).
            Fresh,
            /// The data storage is corrupt or holds no data.
            /// We cannot recover from this state.
            MissingData,
            /// The last run was successfully finished,
            /// so we cannot resume from that run.
            Finished,
            /// The last run was presumably aborted,
            /// so the storages may be in an inconsistent state.
            /// This happens when:
            /// * the scheduler was stopped abruptly (due to external reasons),
            /// * the scheduler was aborted (i.e. due to an error or a `quit -f` signal),
            /// * the data storage does not support graceful recovery,
            /// * or the data storage is volatile or was changed in between runs.
            ///
            /// Recovery is possible if the "done" data is still available,
            /// so an additional check is needed to determine
            /// whether at least the "done" data is available.
            /// Doing so will resolve this state to `MissingData` or `AbortedChecked`.
            AbortedUnchecked,
            /// The last run was aborted, but both storages agree about the "done" data.
            /// This means that we can rebuild the job queues from
            /// the "done" tickets and all resolutions.
            AbortedChecked,
            /// The last run was gracefully stopped,
            /// so we can trust all the metadata in the storage.
            GracefullyStopped,
            /// The last run was gracefully stopped,
            /// and the data storage was additionally checked for consistency.
            GracefullyStoppedChecked,
        }

        type PeerEventReceiver = ::tokio::sync::mpsc::Receiver<PeerEvent>;
        type IntEventSender = ::tokio::sync::mpsc::UnboundedSender<InternalEvent>;
        type IntEventReceiver = ::tokio::sync::mpsc::UnboundedReceiver<InternalEvent>;
        type ControlEventReceiver = ::tokio::sync::watch::Receiver<ControlEvent>;
        type RecoveryStateSender = ::tokio::sync::watch::Sender<RecoveryState>;
        #[derive(Debug, Clone)]
        enum PeerEventSender {
            Up(::tokio::sync::mpsc::Sender<PeerEvent>),
            Downgraded(::tokio::sync::mpsc::WeakSender<PeerEvent>),
        }
        impl PeerEventSender {
            pub async fn send(&self, event: PeerEvent) -> Result<()> {
                match self {
                    PeerEventSender::Up(tx) => tx.send(event).await.map_err(scheduler_error),
                    PeerEventSender::Downgraded(tx) => Err(scheduler_error_str(
                        "Tried to send a PeerEvent through a downgraded sender",
                    )),
                }
            }
            pub fn downgrade(&mut self) {
                match self {
                    PeerEventSender::Up(tx) => {
                        let weak = tx.downgrade();
                        *self = PeerEventSender::Downgraded(weak);
                    }
                    PeerEventSender::Downgraded(_) => {}
                }
            }
        }
        #[derive(Debug, Clone)]
        struct PeerEventSenders {
            to_beta: PeerEventSender,
            to_gamma: PeerEventSender,
            to_delta: PeerEventSender,
            to_epsilon: PeerEventSender,
            to_zeta: PeerEventSender,
        }
        impl PeerEventSenders {
            pub fn downgrade_all(&mut self) {
                self.to_beta.downgrade();
                self.to_gamma.downgrade();
                self.to_delta.downgrade();
                self.to_epsilon.downgrade();
                self.to_zeta.downgrade();
            }
        }
        mod individual_scheduler {
            use super::*;
            /// # IndividualSchedulerOps trait
            ///
            /// This trait defines the event operations that an individual scheduler must implement.
            #[::async_trait::async_trait]
            pub trait IndividualSchedulerOps: Send + Sync + 'static {
                /// Close peer senders that aren't used by this scheduler.
                fn downgrade_unused_peer_senders(&mut self);

                /// Handle a job event.
                async fn on_receive_job<Cl>(
                    &mut self,
                    conn: MetaStorageConnection<'_, Cl>,
                    job: job::Job,
                ) -> Result<()>
                where
                    Cl: MetaClient;
                /// Handle a resolution event.
                async fn on_receive_resolution<Cl>(
                    &mut self,
                    conn: MetaStorageConnection<'_, Cl>,
                    resolution: masked_dimension::Resolution,
                ) -> Result<()>
                where
                    Cl: MetaClient;
                /// Update the UI state ticket counts associated with this scheduler.
                /// Called whenever the UI state information may be changed.
                async fn update_ui<Cl>(
                    &mut self,
                    conn: MetaStorageConnection<'_, Cl>,
                    returning: bool,
                ) -> Result<()>
                where
                    Cl: MetaClient;

                /// Send out the events associated with the just processed job.
                ///
                /// NOTE that peer receivers might be dropped due to a stop signal or an error,
                /// so this function should not error out if the receiver is gone.
                async fn send_on_finish(
                    &self,
                    job: job::Job,
                    resolution: Option<masked_dimension::Resolution>,
                ) -> Result<()>;
            }

            /// # IndividualScheduler
            ///
            /// IndividualScheduler is a scheduler for a single job type.
            /// It is responsible for:
            ///
            /// * Keeping track of the tickets that are ready to run,
            /// * Scheduling the jobs through a semaphore pool,
            /// * Picking up job results and sending out `Event` messages, and
            /// * Updating waiting tickets from `Event` messages.
            ///
            /// Each individual scheduler conceptually "owns" a table in the ticket storage.
            pub struct IndividualScheduler<T: Ticket> {
                ready_to_run: ::std::collections::VecDeque<T>,
                meta_pool: ::deadpool_postgres::Pool,
                meta_schema: Option<String>,
                pool: ::std::sync::Arc<::tokio::sync::Semaphore>,
                pool_size: usize,
                ui_state: ::std::sync::Arc<::tokio::sync::RwLock<ui::UiState>>,
                state: RunningState,
                got_all_updates: bool,
                peer_txs: PeerEventSenders,
                peer_rx: PeerEventReceiver,
                int_tx: IntEventSender,
                int_rx: IntEventReceiver,
                ctrl_rx: ControlEventReceiver,
            }
            impl<T> IndividualScheduler<T>
            where
                T: Ticket,
                Self: IndividualSchedulerOps,
            {
                pub(super) fn new(
                    meta_pool: ::deadpool_postgres::Pool,
                    meta_schema: Option<String>,
                    pool_size: usize,
                    ui_state: ::std::sync::Arc<::tokio::sync::RwLock<ui::UiState>>,
                    peer_txs: PeerEventSenders,
                    peer_rx: PeerEventReceiver,
                    ctrl_rx: ControlEventReceiver,
                ) -> Self {
                    // Create an internal channel for `InternalEvent`s.
                    let (int_tx, int_rx) =
                        ::tokio::sync::mpsc::unbounded_channel::<InternalEvent>();
                    // Downgrade unused peer senders to weak senders.
                    let mut new_self = Self {
                        ready_to_run: ::std::collections::VecDeque::new(),
                        meta_pool,
                        meta_schema,
                        pool: ::std::sync::Arc::new(::tokio::sync::Semaphore::new(pool_size)),
                        pool_size,
                        ui_state,
                        state: RunningState::Running,
                        got_all_updates: false,
                        peer_txs,
                        peer_rx,
                        int_tx,
                        int_rx,
                        ctrl_rx,
                    };
                    new_self.downgrade_unused_peer_senders();
                    new_self
                }

                /// Call `update_ui` without an ongoing connection.
                async fn update_ui_no_conn(&mut self, returning: bool) -> Result<()> {
                    let client = self.meta_pool.get().await.map_err(meta_storage_error)?;
                    let conn = MetaStorageConnection {
                        client: &client,
                        schema: &self.meta_schema.clone(),
                    };
                    self.update_ui(conn, returning).await?;
                    Ok(())
                }

                /// Handle a received event.
                ///
                /// One event corresponds to one metadata transaction.
                async fn on_receive_peer_event(&mut self, event: PeerEvent) -> Result<()> {
                    let mut client = self.meta_pool.get().await.map_err(meta_storage_error)?;
                    let tx = client.transaction().await.map_err(meta_storage_error)?;
                    let conn = MetaStorageConnection {
                        client: &tx,
                        schema: &self.meta_schema.clone(),
                    };
                    match event {
                        PeerEvent::Job(job) => {
                            self.on_receive_job(conn, job).await?;
                        }
                        PeerEvent::Resolution(resolution) => {
                            self.on_receive_resolution(conn, resolution).await?;
                        }
                    }
                    self.update_ui(conn, false).await?;
                    tx.commit().await.map_err(meta_storage_error)?;
                    Ok(())
                }

                /// Drive the scheduler until every ticket of this job type is finished
                /// **and** the broadcast channel has closed.
                ///
                /// Usually called by the top-level `Scheduler::run` with `tokio::spawn`.
                pub async fn run<Sto, Svc>(
                    &mut self,
                    storage: ::std::sync::Arc<Sto>,
                    service: ::std::sync::Arc<Svc>,
                    initial_data: Vec<T>,
                ) -> RunningState
                where
                    Sto: OperonStorage,
                    Svc: OperonService,
                {
                    // Restore the tickets into the ready to run queue.
                    for ticket in initial_data {
                        if ticket.is_ready() {
                            self.ready_to_run.push_back(ticket);
                        } else {
                            error!(
                                "Restored ticket for `{}` job is not ready to run: {ticket:?}",
                                T::job_type()
                            );
                            self.state = RunningState::Error;
                            return self.state;
                        }
                    }
                    // Update the UI state before entering the loop.
                    if self.update_ui_no_conn(false).await.is_err() {
                        error!(
                            "Failed to update UI state for `{}` scheduler after initial data processing.",
                            T::job_type()
                        );
                        self.state = RunningState::Error;
                        return self.state;
                    }
                    // Early return if the scheduler is already finished (e.g. the last run completed this job).
                    if self.state == RunningState::Finished {
                        debug!(
                            "Scheduler for `{}` exited due to being finished from the start.",
                            T::job_type()
                        );
                        self.update_ui_no_conn(true).await.unwrap_or_else(|e| {
                            error!("Failed to update UI state after scheduler run: {e}");
                            self.state = RunningState::Error;
                        });
                        return self.state;
                    }
                    let res = self.run_internal(storage, service).await;
                    // Close peer senders
                    self.peer_txs.downgrade_all();
                    match &res {
                        Ok(()) => {
                            match self.state {
                                RunningState::Finished => {
                                    debug!("Scheduler for `{}` exited normally.", T::job_type())
                                }
                                RunningState::Stopped => {
                                    debug!(
                                        "Scheduler for `{}` was stopped and exited.",
                                        T::job_type()
                                    )
                                }
                                _ => {
                                    error!(
                                        "Scheduler for `{}` exited with an unexpected state: {:?}",
                                        T::job_type(),
                                        self.state
                                    );
                                    self.state = RunningState::Error;
                                }
                            };
                        }
                        Err(e) => {
                            error!(
                                "Scheduler for `{}` exited with an error: {e}",
                                T::job_type()
                            );
                            self.state = RunningState::Error;
                        }
                    }
                    // Update the UI state one last time.
                    self.update_ui_no_conn(true).await.unwrap_or_else(|e| {
                        error!("Failed to update UI state after scheduler run: {e}");
                        self.state = RunningState::Error;
                    });
                    self.state
                }
                async fn run_internal<Sto, Svc>(
                    &mut self,
                    storage: ::std::sync::Arc<Sto>,
                    service: ::std::sync::Arc<Svc>,
                ) -> Result<()>
                where
                    Sto: OperonStorage,
                    Svc: OperonService,
                {
                    let pool = self.pool.clone();
                    // Main event loop.
                    loop {
                        // This loop cannot be entered with the `Error` or `Stopped` state,
                        // as changing the state to `Error` or `Stopped` always exits `run_internal` immediately.
                        // 0. Check the control channel.
                        let ctrl_event = self.ctrl_rx.borrow_and_update().clone();
                        match ctrl_event {
                            ControlEvent::Pause { targets, cascade } => {
                                if targets.is_empty()
                                    || targets.contains(&T::job_type())
                                    || (cascade
                                        && targets
                                            .iter()
                                            .any(|t| T::job_type().is_descendant_of(t)))
                                {
                                    match self.state {
                                        RunningState::Running => {
                                            info!("Pausing `{}` jobs.", T::job_type());
                                            self.state = RunningState::Paused;
                                            self.update_ui_no_conn(false).await?;
                                            // Acquire and forget all permits.
                                            let permit = self
                                                .pool
                                                .clone()
                                                .acquire_many_owned(self.pool_size as u32)
                                                .await
                                                .map_err(scheduler_error)?;
                                            permit.forget();
                                            debug!(
                                                "Remaining `{}` jobs were finished.",
                                                T::job_type()
                                            );
                                        }
                                        RunningState::Paused => {
                                            // Silent no-op, already paused.
                                        }
                                        RunningState::Finished => {
                                            // No jobs to pause, no-op.
                                            // This scheduler is executing the leftover `send_on_finish` events,
                                            // which are not affected by the pause.
                                        }
                                        _ => {
                                            error!(
                                                "Scheduler for `{}` entered event loop in an unexpected state: {:?}",
                                                T::job_type(),
                                                self.state
                                            );
                                            self.state = RunningState::Error;
                                            return Err(scheduler_error_str(
                                                "Scheduler entered event loop in an unexpected state",
                                            ));
                                        }
                                    }
                                }
                            }
                            ControlEvent::Resume { targets } => {
                                if targets.is_empty() || targets.contains(&T::job_type()) {
                                    match self.state {
                                        RunningState::Paused => {
                                            info!("Resuming `{}` jobs.", T::job_type());
                                            self.state = RunningState::Running;
                                            self.update_ui_no_conn(false).await?;
                                            // Add back all permits.
                                            self.pool.add_permits(self.pool_size);
                                        }
                                        RunningState::Running => {
                                            // Silent no-op, already running.
                                        }
                                        RunningState::Finished => {
                                            // No jobs to resume, no-op.
                                        }
                                        _ => {
                                            error!(
                                                "Scheduler for `{}` entered event loop in an unexpected state: {:?}",
                                                T::job_type(),
                                                self.state
                                            );
                                            self.state = RunningState::Error;
                                            return Err(scheduler_error_str(
                                                "Scheduler entered event loop in an unexpected state",
                                            ));
                                        }
                                    }
                                }
                            }
                            ControlEvent::GracefulStop => {
                                if self.state == RunningState::Running {
                                    info!("Pausing `{}` jobs for graceful stop.", T::job_type());
                                    self.state = RunningState::Paused;
                                    self.update_ui_no_conn(false).await?;
                                    // Acquire and forget all permits.
                                    let permit = self
                                        .pool
                                        .clone()
                                        .acquire_many_owned(self.pool_size as u32)
                                        .await
                                        .map_err(scheduler_error)?;
                                    permit.forget();
                                    debug!("Remaining `{}` jobs were finished.", T::job_type());
                                }
                                match self.state {
                                    RunningState::Paused | RunningState::Finished => {
                                        if self.int_rx.is_empty() && self.got_all_updates {
                                            info!("Gracefully stopped `{}` jobs.", T::job_type());
                                            // If the state is `Paused`, set it to `Stopped`,
                                            // If the state is `Finished`, keep it as `Finished`.
                                            if self.state == RunningState::Paused {
                                                self.state = RunningState::Stopped;
                                            }
                                            return Ok(());
                                        } else {
                                            // No-op, we still have some internal events to process.
                                        }
                                    }
                                    _ => {
                                        error!(
                                            "Scheduler for `{}` entered event loop in an unexpected state: {:?}",
                                            T::job_type(),
                                            self.state
                                        );
                                        self.state = RunningState::Error;
                                        return Err(scheduler_error_str(
                                            "Scheduler entered event loop in an unexpected state",
                                        ));
                                    }
                                }
                            }
                            ControlEvent::Abort => {
                                info!("Aborting `{}` jobs.", T::job_type());
                                // If this is a finished scheduler rolling out peer events,
                                // don't change the state to `Stopped`,
                                // since it is already `Finished`.
                                if matches!(
                                    self.state,
                                    RunningState::Running | RunningState::Paused
                                ) {
                                    self.state = RunningState::Stopped;
                                }
                                return Ok(());
                            }
                            _ => {
                                // No-op for other control events.
                            }
                        }

                        ::tokio::select! {
                            // 0. Also wait on the control channel to avoid being stuck in this loop.
                            ctrl_event = self.ctrl_rx.changed() => {
                                ctrl_event.map_err(scheduler_error)?;
                            }

                            // 1. An internal event.
                            int_event = self.int_rx.recv() => {
                                let int_event = int_event.ok_or(
                                    scheduler_error_str("Scheduler internal channel closed prematurely")
                                )?;
                                self.update_ui_no_conn(false).await?;
                                match int_event {
                                    InternalEvent::JobSuccess(job, resolution) => {
                                        // Trace the job success
                                        trace!(
                                            "{} received internal event: JobSuccess({job:?}, {resolution:?}); \
                                            Internal channel has {} events left.",
                                            T::job_type(), self.int_rx.len()
                                        );

                                        // Broadcast the job result events
                                        self.send_on_finish(job, resolution).await?;
                                        // If all the tickets are finished
                                        // AND the scheduler's internal events are drained,
                                        // exit the loop.
                                        if self.state == RunningState::Finished && self.int_rx.is_empty(){
                                            return Ok(());
                                        }
                                    }
                                    InternalEvent::JobFailure(job, e) => {
                                        // Log the error
                                        error!("Job {job:?} failed: {e}");
                                        // Return the error to the top-level scheduler
                                        return Err(e);
                                    }
                                }
                            }

                            // 2. A peer event.
                            event = self.peer_rx.recv(), if !self.got_all_updates => {
                                match event {
                                    Some(event) => {
                                        // Trace the peer event
                                        trace!(
                                            "{} received peer event: {event:?}; \
                                            Peer channel has {} events left.",
                                            T::job_type(), self.peer_rx.len()
                                        );
                                        self.on_receive_peer_event(event).await?
                                    },
                                    None => {
                                        // The peer channel was closed,
                                        // meaning that all peer updates were received,
                                        // or that the upstream scheduler was gracefully stopped.
                                        // Either way, we stop listening this branch.
                                        debug!("`{}` finished receiving updates.", T::job_type());
                                        self.got_all_updates = true;
                                    }
                                }
                            }

                            // 3. Run a job.
                            // If the scheduler is paused, the pool will not yield a permit
                            // since the pool will have forgotten the permits.
                            permit = pool.clone().acquire_owned(),
                                if !self.ready_to_run.is_empty()
                            => {
                                let permit = permit.map_err(scheduler_error)?;
                                let ticket = self.ready_to_run.pop_front().ok_or(scheduler_error_str("Ready to run queue is empty"))?;
                                let job = ticket.resolve().ok_or(scheduler_error_str("Ticket is not ready to run"))?;
                                let storage = storage.clone();
                                let service = service.clone();
                                let meta_pool = self.meta_pool.clone();
                                let meta_schema = self.meta_schema.clone();
                                let int_sender = self.int_tx.clone();

                                // Move the permit into the task so it is released on drop.
                                // The metadata storage operations are grouped in one transaction here.
                                ::tokio::spawn(async move {
                                    // Trace the job start.
                                    trace!("Running job {job:?} in `{}` scheduler.", T::job_type());
                                    let _permit = permit;
                                    let mut client = meta_pool.get().await.map_err(meta_storage_error)?;
                                    let tx = client.transaction().await.map_err(meta_storage_error)?;
                                    let conn = MetaStorageConnection {
                                        client: &tx,
                                        schema: &meta_schema,
                                    };
                                    match job.clone().run_job(conn, &*storage, &*service).await {
                                        Ok(resolution) => {
                                            // Mark the ticket as done in the ticket storage
                                            tickets_psql::mark_done(conn, &job).await?;
                                            // Put the resolution in the fact storage
                                            if let Some(resolution) = &resolution {
                                                facts_psql::put_resolution(conn, resolution).await?;
                                            }
                                            tx.commit().await.map_err(meta_storage_error)?;

                                            // Alert the results to the scheduler
                                            int_sender.send(InternalEvent::JobSuccess(job.clone(), resolution.clone()))
                                                .map_err(|e| scheduler_error_str(format!("Failed to send internal event: {e}")))?;
                                            trace!(
                                                "{} sent internal event: JobSuccess({job:?}, {resolution:?});",
                                                T::job_type()
                                            );

                                            Ok(())
                                        }
                                        Err(e) => {
                                            // Rollback the transaction
                                            tx.rollback().await.map_err(meta_storage_error)?;

                                            // Alert the error to the scheduler
                                            int_sender.send(InternalEvent::JobFailure(job, e))
                                                .map_err(|e| scheduler_error_str(format!("Failed to send internal event: {e}")))?;
                                            Err(scheduler_error_str("Job failed"))
                                        }
                                    }
                                });
                            }
                        }
                    }
                }
            }
            #[::async_trait::async_trait]
            impl IndividualSchedulerOps for IndividualScheduler<BetaTicket> {
                fn downgrade_unused_peer_senders(&mut self) {
                    // Beta scheduler uses delta and epsilon peer senders.
                    self.peer_txs.to_beta.downgrade();
                    self.peer_txs.to_gamma.downgrade();
                    self.peer_txs.to_zeta.downgrade();
                }

                async fn on_receive_job<Cl>(
                    &mut self,
                    conn: MetaStorageConnection<'_, Cl>,
                    job: job::Job,
                ) -> Result<()>
                where
                    Cl: MetaClient,
                {
                    // Beta jobs don't have dependencies.
                    match job {
                        _ => Err(scheduler_error_str(
                            "Irrelevant job event received in beta scheduler",
                        )),
                    }
                }
                async fn on_receive_resolution<Cl>(
                    &mut self,
                    conn: MetaStorageConnection<'_, Cl>,
                    resolution: masked_dimension::Resolution,
                ) -> Result<()>
                where
                    Cl: MetaClient,
                {
                    match &resolution {
                        masked_dimension::Resolution::I(_) => {
                            let tickets = tickets_psql::explode_beta(conn, &resolution).await?;
                            self.ready_to_run.extend(tickets);
                            Ok(())
                        }
                        _ => Err(scheduler_error_str(
                            "Irrelevant resolution event received in beta scheduler",
                        )),
                    }
                }
                async fn update_ui<Cl>(
                    &mut self,
                    conn: MetaStorageConnection<'_, Cl>,
                    returning: bool,
                ) -> Result<()>
                where
                    Cl: MetaClient,
                {
                    let counts = tickets_psql::get_beta_status(conn).await?;
                    if counts.1 + counts.2 == 0 && self.state != RunningState::Finished {
                        // Finished all jobs.
                        info!("All `beta` jobs finished successfully.");
                        self.state = RunningState::Finished;
                    }
                    let update = ui::UiStateUpdate::Beta((
                        counts.0, counts.1, counts.2, self.state, returning,
                    ));
                    ui::update_ui_state(&self.ui_state.clone(), update).await;
                    Ok(())
                }
                async fn send_on_finish(
                    &self,
                    job: job::Job,
                    resolution: Option<masked_dimension::Resolution>,
                ) -> Result<()> {
                    let Some(resolution) = resolution else {
                        return Err(scheduler_error_str(
                            "Beta job finished without a resolution",
                        ));
                    };
                    // j -> delta
                    self.peer_txs
                        .to_delta
                        .send(PeerEvent::Resolution(resolution.clone()))
                        .await
                        .unwrap_or_else(|_| {
                            // Verbosity should be low here, since this can happen
                            // an arbitrary number of times
                            // if a descendant scheduler errored out.
                            trace!(
                                "`delta`'s peer channel closed before handling `beta`'s {resolution:?}"
                            );
                        });
                    trace!("`beta` sent peer event to `delta`: {resolution:?}");
                    // out-dependencies (delta, epsilon)
                    self.peer_txs
                        .to_delta
                        .send(PeerEvent::Job(job.clone()))
                        .await
                        .unwrap_or_else(|_| {
                            trace!(
                                "`delta`'s peer channel closed before handling `beta`'s {job:?}"
                            );
                        });
                    trace!("`beta` sent peer event to `delta`: {job:?}");
                    self.peer_txs
                        .to_epsilon
                        .send(PeerEvent::Job(job.clone()))
                        .await
                        .unwrap_or_else(|_| {
                            trace!(
                                "`epsilon`'s peer channel closed before handling `beta`'s {job:?}"
                            );
                        });
                    trace!("`beta` sent peer event to `epsilon`: {job:?}");
                    Ok(())
                }
            }
            #[::async_trait::async_trait]
            impl IndividualSchedulerOps for IndividualScheduler<GammaTicket> {
                fn downgrade_unused_peer_senders(&mut self) {
                    // Gamma scheduler uses delta, epsilon, and zeta peer senders.
                    self.peer_txs.to_beta.downgrade();
                    self.peer_txs.to_gamma.downgrade();
                }

                async fn on_receive_job<Cl>(
                    &mut self,
                    conn: MetaStorageConnection<'_, Cl>,
                    job: job::Job,
                ) -> Result<()>
                where
                    Cl: MetaClient,
                {
                    // Gamma jobs don't have dependencies.
                    match job {
                        _ => Err(scheduler_error_str(
                            "Irrelevant job event received in gamma scheduler",
                        )),
                    }
                }
                async fn on_receive_resolution<Cl>(
                    &mut self,
                    conn: MetaStorageConnection<'_, Cl>,
                    resolution: masked_dimension::Resolution,
                ) -> Result<()>
                where
                    Cl: MetaClient,
                {
                    match &resolution {
                        masked_dimension::Resolution::I(_) => {
                            let tickets = tickets_psql::explode_gamma(conn, &resolution).await?;
                            self.ready_to_run.extend(tickets);
                            Ok(())
                        }
                        _ => Err(scheduler_error_str(
                            "Irrelevant resolution event received in gamma scheduler",
                        )),
                    }
                }
                async fn update_ui<Cl>(
                    &mut self,
                    conn: MetaStorageConnection<'_, Cl>,
                    returning: bool,
                ) -> Result<()>
                where
                    Cl: MetaClient,
                {
                    let counts = tickets_psql::get_gamma_status(conn).await?;
                    if counts.1 + counts.2 == 0 && self.state != RunningState::Finished {
                        info!("All `gamma` jobs finished successfully.");
                        self.state = RunningState::Finished;
                    }
                    let update = ui::UiStateUpdate::Gamma((
                        counts.0, counts.1, counts.2, self.state, returning,
                    ));
                    ui::update_ui_state(&self.ui_state.clone(), update).await;
                    Ok(())
                }
                async fn send_on_finish(
                    &self,
                    job: job::Job,
                    resolution: Option<masked_dimension::Resolution>,
                ) -> Result<()> {
                    let Some(resolution) = resolution else {
                        return Err(scheduler_error_str(
                            "Gamma job finished without a resolution",
                        ));
                    };
                    // k -> delta, epsilon
                    self.peer_txs
                    .to_delta
                    .send(PeerEvent::Resolution(resolution.clone()))
                    .await
                    .unwrap_or_else(|_| {
                        trace!(
                            "`delta`'s peer channel closed before handling `gamma`'s {resolution:?}"
                        );
                    });
                    trace!("`gamma` sent peer event to `delta`: {resolution:?}");
                    self.peer_txs
                        .to_epsilon
                        .send(PeerEvent::Resolution(resolution.clone()))
                        .await
                        .unwrap_or_else(|_| {
                            trace!("`epsilon`'s peer channel closed before handling `gamma`'s {resolution:?}");
                        });
                    trace!("`gamma` sent peer event to `epsilon`: {resolution:?}");
                    // out-dependencies (delta, zeta)
                    self.peer_txs
                        .to_delta
                        .send(PeerEvent::Job(job.clone()))
                        .await
                        .unwrap_or_else(|_| {
                            trace!(
                                "`delta`'s peer channel closed before handling `gamma`'s {job:?}"
                            );
                        });
                    trace!("`gamma` sent peer event to `delta`: {job:?}");
                    self.peer_txs
                        .to_zeta
                        .send(PeerEvent::Job(job.clone()))
                        .await
                        .unwrap_or_else(|_| {
                            trace!(
                                "`zeta`'s peer channel closed before handling `gamma`'s {job:?}"
                            );
                        });
                    trace!("`gamma` sent peer event to `zeta`: {job:?}");
                    Ok(())
                }
            }
            #[::async_trait::async_trait]
            impl IndividualSchedulerOps for IndividualScheduler<DeltaTicket> {
                fn downgrade_unused_peer_senders(&mut self) {
                    // Delta scheduler uses the epsilon peer sender.
                    self.peer_txs.to_beta.downgrade();
                    self.peer_txs.to_gamma.downgrade();
                    self.peer_txs.to_delta.downgrade();
                    self.peer_txs.to_zeta.downgrade();
                }

                async fn on_receive_job<Cl>(
                    &mut self,
                    conn: MetaStorageConnection<'_, Cl>,
                    job: job::Job,
                ) -> Result<()>
                where
                    Cl: MetaClient,
                {
                    // delta_i,j,k depends on beta_i and gamma_i.
                    match job {
                        job::Job::Beta { i } => {
                            let tickets = tickets_psql::raise_dep_delta(
                                conn,
                                &masked_dimension::I::One(i),
                                &masked_dimension::J::All(masked_dimension::I::One(i)),
                                &masked_dimension::K::All(masked_dimension::I::One(i)),
                            )
                            .await?;
                            self.ready_to_run.extend(tickets);
                            Ok(())
                        }
                        job::Job::Gamma { i } => {
                            let tickets = tickets_psql::raise_dep_delta(
                                conn,
                                &masked_dimension::I::One(i),
                                &masked_dimension::J::All(masked_dimension::I::One(i)),
                                &masked_dimension::K::All(masked_dimension::I::One(i)),
                            )
                            .await?;
                            self.ready_to_run.extend(tickets);
                            Ok(())
                        }
                        _ => Err(scheduler_error_str(
                            "Irrelevant job event received in delta scheduler",
                        )),
                    }
                }
                async fn on_receive_resolution<Cl>(
                    &mut self,
                    conn: MetaStorageConnection<'_, Cl>,
                    resolution: masked_dimension::Resolution,
                ) -> Result<()>
                where
                    Cl: MetaClient,
                {
                    match &resolution {
                        masked_dimension::Resolution::I(_)
                        | masked_dimension::Resolution::J(_, _)
                        | masked_dimension::Resolution::K(_, _) => {
                            let tickets = tickets_psql::explode_delta(conn, &resolution).await?;
                            self.ready_to_run.extend(tickets);
                            Ok(())
                        }
                        _ => Err(scheduler_error_str(
                            "Irrelevant resolution event received in delta scheduler",
                        )),
                    }
                }
                async fn update_ui<Cl>(
                    &mut self,
                    conn: MetaStorageConnection<'_, Cl>,
                    returning: bool,
                ) -> Result<()>
                where
                    Cl: MetaClient,
                {
                    let counts = tickets_psql::get_delta_status(conn).await?;
                    if counts.1 + counts.2 == 0 && self.state != RunningState::Finished {
                        info!("All `delta` jobs finished successfully.");
                        self.state = RunningState::Finished;
                    }
                    let update = ui::UiStateUpdate::Delta((
                        counts.0, counts.1, counts.2, self.state, returning,
                    ));
                    ui::update_ui_state(&self.ui_state.clone(), update).await;
                    Ok(())
                }
                async fn send_on_finish(
                    &self,
                    job: job::Job,
                    resolution: Option<masked_dimension::Resolution>,
                ) -> Result<()> {
                    if resolution.is_some() {
                        return Err(scheduler_error_str("Delta job finished with a resolution"));
                    }
                    // out-dependencies (epsilon)
                    self.peer_txs
                        .to_epsilon
                        .send(PeerEvent::Job(job.clone()))
                        .await
                        .unwrap_or_else(|_| {
                            trace!(
                                "`epsilon`'s peer channel closed before handling `delta`'s {job:?}"
                            );
                        });
                    trace!("`delta` sent peer event to `epsilon`: {job:?}");
                    Ok(())
                }
            }
            #[::async_trait::async_trait]
            impl IndividualSchedulerOps for IndividualScheduler<EpsilonTicket> {
                fn downgrade_unused_peer_senders(&mut self) {
                    // Epsilon scheduler uses the zeta peer sender.
                    self.peer_txs.to_beta.downgrade();
                    self.peer_txs.to_gamma.downgrade();
                    self.peer_txs.to_delta.downgrade();
                    self.peer_txs.to_epsilon.downgrade();
                }

                async fn on_receive_job<Cl>(
                    &mut self,
                    conn: MetaStorageConnection<'_, Cl>,
                    job: job::Job,
                ) -> Result<()>
                where
                    Cl: MetaClient,
                {
                    // epsilon_i,k depends on beta_i and delta_i,j,k.
                    match job {
                        job::Job::Beta { i } => {
                            let tickets = tickets_psql::raise_dep_epsilon(
                                conn,
                                &masked_dimension::I::One(i),
                                &masked_dimension::K::All(masked_dimension::I::One(i)),
                            )
                            .await?;
                            self.ready_to_run.extend(tickets);
                            Ok(())
                        }
                        job::Job::Delta { i, j, k } => {
                            let tickets = tickets_psql::raise_dep_epsilon(
                                conn,
                                &masked_dimension::I::One(i),
                                &masked_dimension::K::One(k),
                            )
                            .await?;
                            self.ready_to_run.extend(tickets);
                            Ok(())
                        }
                        _ => Err(scheduler_error_str(
                            "Irrelevant job event received in epsilon scheduler",
                        )),
                    }
                }
                async fn on_receive_resolution<Cl>(
                    &mut self,
                    conn: MetaStorageConnection<'_, Cl>,
                    resolution: masked_dimension::Resolution,
                ) -> Result<()>
                where
                    Cl: MetaClient,
                {
                    match &resolution {
                        masked_dimension::Resolution::I(_)
                        | masked_dimension::Resolution::K(_, _) => {
                            let tickets = tickets_psql::explode_epsilon(conn, &resolution).await?;
                            self.ready_to_run.extend(tickets);
                            Ok(())
                        }
                        _ => Err(scheduler_error_str(
                            "Irrelevant resolution event received in epsilon scheduler",
                        )),
                    }
                }
                async fn update_ui<Cl>(
                    &mut self,
                    conn: MetaStorageConnection<'_, Cl>,
                    returning: bool,
                ) -> Result<()>
                where
                    Cl: MetaClient,
                {
                    let counts = tickets_psql::get_epsilon_status(conn).await?;
                    if counts.1 + counts.2 == 0 && self.state != RunningState::Finished {
                        info!("All `epsilon` jobs finished successfully.");
                        self.state = RunningState::Finished;
                    }
                    let update = ui::UiStateUpdate::Epsilon((
                        counts.0, counts.1, counts.2, self.state, returning,
                    ));
                    ui::update_ui_state(&self.ui_state.clone(), update).await;
                    Ok(())
                }
                async fn send_on_finish(
                    &self,
                    job: job::Job,
                    resolution: Option<masked_dimension::Resolution>,
                ) -> Result<()> {
                    if resolution.is_some() {
                        return Err(scheduler_error_str(
                            "Epsilon job finished with a resolution",
                        ));
                    }
                    // out-dependencies (zeta)
                    self.peer_txs
                        .to_zeta
                        .send(PeerEvent::Job(job.clone()))
                        .await
                        .unwrap_or_else(|_| {
                            trace!(
                                "`zeta`'s peer channel closed before handling `epsilon`'s {job:?}."
                            );
                        });
                    trace!("`epsilon` sent peer event to `zeta`: {job:?}");
                    Ok(())
                }
            }
            #[::async_trait::async_trait]
            impl IndividualSchedulerOps for IndividualScheduler<ZetaTicket> {
                fn downgrade_unused_peer_senders(&mut self) {
                    // Zeta scheduler uses no peer senders.
                    self.peer_txs.to_beta.downgrade();
                    self.peer_txs.to_gamma.downgrade();
                    self.peer_txs.to_delta.downgrade();
                    self.peer_txs.to_epsilon.downgrade();
                    self.peer_txs.to_zeta.downgrade();
                }

                async fn on_receive_job<Cl>(
                    &mut self,
                    conn: MetaStorageConnection<'_, Cl>,
                    job: job::Job,
                ) -> Result<()>
                where
                    Cl: MetaClient,
                {
                    // zeta_i depends on gamma_i and epsilon_i,k.
                    match job {
                        job::Job::Gamma { i } => {
                            let tickets =
                                tickets_psql::raise_dep_zeta(conn, &masked_dimension::I::One(i))
                                    .await?;
                            self.ready_to_run.extend(tickets);
                            Ok(())
                        }
                        job::Job::Epsilon { i, k } => {
                            let tickets =
                                tickets_psql::raise_dep_zeta(conn, &masked_dimension::I::One(i))
                                    .await?;
                            self.ready_to_run.extend(tickets);
                            Ok(())
                        }
                        _ => Err(scheduler_error_str(
                            "Irrelevant job event received in zeta scheduler",
                        )),
                    }
                }
                async fn on_receive_resolution<Cl>(
                    &mut self,
                    conn: MetaStorageConnection<'_, Cl>,
                    resolution: masked_dimension::Resolution,
                ) -> Result<()>
                where
                    Cl: MetaClient,
                {
                    match &resolution {
                        masked_dimension::Resolution::I(_) => {
                            let tickets = tickets_psql::explode_zeta(conn, &resolution).await?;
                            self.ready_to_run.extend(tickets);
                            Ok(())
                        }
                        _ => Err(scheduler_error_str(
                            "Irrelevant resolution event received in zeta scheduler",
                        )),
                    }
                }
                async fn update_ui<Cl>(
                    &mut self,
                    conn: MetaStorageConnection<'_, Cl>,
                    returning: bool,
                ) -> Result<()>
                where
                    Cl: MetaClient,
                {
                    let counts = tickets_psql::get_zeta_status(conn).await?;
                    if counts.1 + counts.2 == 0 && self.state != RunningState::Finished {
                        info!("All `zeta` jobs finished successfully.");
                        self.state = RunningState::Finished;
                    }
                    let update = ui::UiStateUpdate::Zeta((
                        counts.0, counts.1, counts.2, self.state, returning,
                    ));
                    ui::update_ui_state(&self.ui_state.clone(), update).await;
                    Ok(())
                }
                async fn send_on_finish(
                    &self,
                    job: job::Job,
                    resolution: Option<masked_dimension::Resolution>,
                ) -> Result<()> {
                    if resolution.is_some() {
                        return Err(scheduler_error_str("Zeta job finished with a resolution"));
                    }
                    // No out-dependencies.
                    Ok(())
                }
            }
        }
        pub use individual_scheduler::*;

        /// # Scheduler
        ///
        /// The orchestrating scheduler that manages the individual schedulers.
        ///
        /// It is responsible for:
        ///
        /// * Initialization of the metadata storage,
        /// * initialization of the individual schedulers, and
        /// * communication between the UI and the individual schedulers.
        pub struct Scheduler<Sto, Svc> {
            storage: ::std::sync::Arc<Sto>,
            service: ::std::sync::Arc<Svc>,
            meta_pool: ::deadpool_postgres::Pool,
            meta_schema: Option<String>,
            ui_state: ::std::sync::Arc<::tokio::sync::RwLock<ui::UiState>>,
            peer_txs: Option<PeerEventSenders>,
            ctrl_rx: ControlEventReceiver,
            rec_tx: RecoveryStateSender,
            handles: ::tokio::task::JoinSet<RunningState>,
        }

        impl<Sto, Svc> Scheduler<Sto, Svc>
        where
            Sto: OperonStorage,
            Svc: OperonService,
        {
            /// Initialize a new scheduler and its associated storages.
            pub async fn new(
                storage: ::std::sync::Arc<Sto>,
                service: ::std::sync::Arc<Svc>,
                ui_state: ::std::sync::Arc<::tokio::sync::RwLock<ui::UiState>>,
                ctrl_rx: ControlEventReceiver,
                rec_tx: RecoveryStateSender,
            ) -> Result<Self> {
                let uri = META_DATABASE_URI;
                let meta_schema = META_SCHEMA.map(|s| s.to_string());

                let meta_pool = init_connection(uri).await?;
                {
                    // Initialize the metadata storage.
                    let mut client = meta_pool.get().await.map_err(meta_storage_error)?;
                    let tx = client.transaction().await.map_err(meta_storage_error)?;
                    let conn = MetaStorageConnection {
                        client: &tx,
                        schema: &meta_schema,
                    };
                    init_schema(conn).await?;
                    facts_psql::init(conn).await?;
                    tickets_psql::init(conn).await?;
                    footprint_psql::init(conn).await?;
                    tx.commit().await.map_err(meta_storage_error)?;
                }
                Ok(Self {
                    storage,
                    service,
                    meta_pool,
                    meta_schema,
                    ui_state,
                    peer_txs: None,
                    ctrl_rx,
                    rec_tx,
                    handles: ::tokio::task::JoinSet::new(),
                })
            }

            /// Main entry point for the scheduler.
            pub async fn work(mut self) -> Result<()> {
                // First, check the recovery state.
                let recovery_state = self.check_recovery_state().await.map_err(|e| {
                    error!("Failed to check the state from last run.");
                    if let Err(e) = self
                        .rec_tx
                        .send(RecoveryState::Error)
                        .map_err(|_| scheduler_error_str("Failed to send recovery state"))
                    {
                        return e;
                    };
                    e
                })?;
                self.rec_tx
                    .send(recovery_state)
                    .map_err(|_| scheduler_error_str("Failed to send recovery state"))?;
                match recovery_state {
                    RecoveryState::Fresh => {
                        info!("Type `run` to begin running jobs.");
                    }
                    RecoveryState::Finished => {
                        info!(
                            "Found a finished run. \n\
                            Type `run` to begin running jobs and overwrite the existing data, \
                            or `exit` to cancel."
                        );
                    }
                    RecoveryState::GracefullyStopped => {
                        info!(
                            "Found a gracefully stopped run. \n\
                            Type `run` to resume running jobs from the last run, \
                            or `help` for additional options."
                        )
                    }
                    RecoveryState::AbortedUnchecked => {
                        info!(
                            "Found an aborted run. \n\
                            Type `check` to check if the data is recoverable, \
                            `run` to start a new run and overwrite the existing data, \
                            or `help` for additional options."
                        );
                    }
                    _ => unreachable!("Unexpected recovery state: {recovery_state:?}"),
                }
                // Then, wait for the UI to decide what to do next.
                loop {
                    self.ctrl_rx.changed().await.map_err(scheduler_error)?;
                    let ctrl_event = self.ctrl_rx.borrow_and_update().clone();
                    match ctrl_event {
                        ControlEvent::Check { primary_ub } => {
                            let consistent =
                                self.check_consistency(primary_ub).await.map_err(|e| {
                                    error!("Failed to check data consistency: {e}");
                                    if let Err(e) =
                                        self.rec_tx.send(RecoveryState::Error).map_err(|_| {
                                            scheduler_error_str("Failed to send recovery state")
                                        })
                                    {
                                        return e;
                                    };
                                    e
                                })?;
                            let state_after_check = if consistent {
                                match recovery_state {
                                    RecoveryState::AbortedUnchecked => {
                                        RecoveryState::AbortedChecked
                                    }
                                    RecoveryState::GracefullyStopped => {
                                        RecoveryState::GracefullyStoppedChecked
                                    }
                                    _ => unreachable!(
                                        "Ran `check_consistency` in an unexpected state: {recovery_state:?}"
                                    ),
                                }
                            } else {
                                RecoveryState::MissingData
                            };
                            self.rec_tx.send(state_after_check).map_err(|_| {
                                scheduler_error_str("Failed to send recovery state")
                            })?;
                            match state_after_check {
                                RecoveryState::MissingData => {
                                    info!(
                                        "Some data is corrupted or missing. \n\
                                        Type `run` to start a new run and overwrite the existing data, \
                                        or `exit` to cancel."
                                    )
                                }
                                RecoveryState::AbortedChecked => {
                                    info!(
                                        "The data is recoverable. \n\
                                        Type `run` to rebuild and resume running jobs from the last run, \
                                        or `help` for additional options."
                                    )
                                }
                                RecoveryState::GracefullyStoppedChecked => {
                                    info!(
                                        "No inconsistencies were found. \n\
                                        Type `run` to resume running jobs from the last run, \
                                        or `help` for additional options."
                                    );
                                }
                                _ => unreachable!(
                                    "Unexpected recovery state after consistency check: {consistent:?}"
                                ),
                            }
                        }
                        ControlEvent::CleanRun { primary_ub } => {
                            return self.run(primary_ub, RunMode::Clean).await;
                        }
                        ControlEvent::RebuildRun { primary_ub } => {
                            return self.run(primary_ub, RunMode::Rebuild).await;
                        }
                        ControlEvent::RestoreRun { primary_ub } => {
                            return self.run(primary_ub, RunMode::Restore).await;
                        }
                        ControlEvent::Abort => {
                            // Decided to not start a new run.
                            return Ok(());
                        }
                        _ => {
                            // Other control events should not be passed in here.
                            warn!("Received an unexpected control event: {ctrl_event:?}");
                            return Err(scheduler_error_str(format!(
                                "Unexpected control event: {ctrl_event:?}"
                            )));
                        }
                    }
                }
            }

            /// Find out the recovery state.
            pub async fn check_recovery_state(&self) -> Result<RecoveryState> {
                // Get footprints from both storages.
                let data_footprint = self
                    .storage
                    .get_footprint()
                    .await
                    .map_err(OperonError::Storage)?;
                let meta_footprint = {
                    let client = self.meta_pool.get().await.map_err(meta_storage_error)?;
                    // Read-only, so no transaction needed.
                    let conn = MetaStorageConnection {
                        client: &client,
                        schema: &self.meta_schema,
                    };
                    footprint_psql::get_footprint(conn, "global".to_string()).await?
                };
                // Early return if the state can be inferred through the footprints.
                match (&data_footprint, &meta_footprint) {
                    (Some(df), Some(mf)) if df == mf && df.starts_with("F@") => {
                        // Both storages have the same footprint, and it is a "finished" one.
                        return Ok(RecoveryState::Finished);
                    }
                    (Some(df), Some(mf)) if df == mf && df.starts_with("S@") => {
                        // Both storages have the same footprint, and it is a "stopped" one.
                        return Ok(RecoveryState::GracefullyStopped);
                    }
                    _ => {}
                }
                // Check for `Fresh`: whether the metadata storage holds the primary resolution.
                let client = self.meta_pool.get().await.map_err(meta_storage_error)?;
                let conn = MetaStorageConnection {
                    client: &client,
                    schema: &self.meta_schema,
                };
                let resolution =
                    facts_psql::get_resolution(conn, &masked_dimension::ResolutionRequest::I)
                        .await?;
                if resolution.is_none() {
                    // No resolution found, so the metadata storage is empty.
                    return Ok(RecoveryState::Fresh);
                }
                // Fall back to `AbortedUnchecked`: the metadata storage has some data,
                // but it is not consistent with the data storage.
                Ok(RecoveryState::AbortedUnchecked)
            }

            /// Run a check on the data consistency between the data storage and the metadata storage.
            /// Return `true` if the data storage holds all needed data to restore,
            /// or `false` if it does not.
            ///
            /// This should be called only when the recovery state is either
            /// `AbortedUnchecked` or `GracefullyStopped`.
            async fn check_consistency(&self, primary_ub: usize) -> Result<bool> {
                '_a: {
                    // Pull the primary resolution from the metadata storage...
                    let client = self.meta_pool.get().await.map_err(meta_storage_error)?;
                    let conn = MetaStorageConnection {
                        client: &client,
                        schema: &self.meta_schema,
                    };
                    let Some(i_ub) =
                        facts_psql::get_resolution(conn, &masked_dimension::ResolutionRequest::I)
                            .await?
                            .and_then(|r| {
                                if let masked_dimension::Resolution::I(i) = r {
                                    Some(i)
                                } else {
                                    None
                                }
                            })
                    else {
                        // This is technically unreachable, because we check this same value
                        // in `check_recovery_state`.
                        info!("No primary resolution found in the metadata storage.");
                        return Ok(false);
                    };
                    // ...and check if the data storage holds all the data for it.
                    for i in 0..i_ub.max(primary_ub) {
                        if self
                            .storage
                            .get_a(i)
                            .await
                            .map_err(OperonError::Storage)?
                            .is_none()
                        {
                            info!("Data storage does not hold `A_{i}`.");
                            return Ok(false);
                        }
                    }
                    // Additionally check if the primary resolution agrees with the given upper bound.
                    if i_ub != primary_ub {
                        warn!(
                            "Previous run's upper bound `{i_ub}` is different from the current run's upper bound `{primary_ub}`. \n\
                            If you overwrote the primary data, consider running `run --fresh` to overwrite the existing data, \
                            otherwise the resulting data may be inconsistent. \n\
                            If you want to keep the existing data, and intendedly set the upper bound to `{primary_ub}`, \
                            you may ignore this warning."
                        );
                    }
                }
                '_b: {
                    // Pull the "done" beta jobs from the metadata storage...
                    let client = self.meta_pool.get().await.map_err(meta_storage_error)?;
                    let conn = MetaStorageConnection {
                        client: &client,
                        schema: &self.meta_schema,
                    };
                    let Some(beta_jobs) = tickets_psql::get_all_done::<_, BetaTicket>(conn)
                        .await?
                        .iter()
                        .map(|t| {
                            t.resolve().and_then(|job| {
                                if let job::Job::Beta { i } = job {
                                    Some((i,))
                                } else {
                                    None
                                }
                            })
                        })
                        .collect::<Option<Vec<_>>>()
                    else {
                        info!("Some `beta` tickets are corrupt in the metadata storage.");
                        return Ok(false);
                    };
                    // ...and map them with the dimensions they spawned...
                    let mut b_tags = vec![];
                    for (i,) in beta_jobs {
                        let Some(j_ub) = facts_psql::get_resolution(
                            conn,
                            &masked_dimension::ResolutionRequest::J(i),
                        )
                        .await?
                        .and_then(|r| {
                            if let masked_dimension::Resolution::J(j_ub, _) = r {
                                Some(j_ub)
                            } else {
                                None
                            }
                        }) else {
                            info!(
                                "No `j` resolution found for `beta_{i}` in the metadata storage."
                            );
                            return Ok(false);
                        };
                        for j in 0..j_ub {
                            b_tags.push((i, j));
                        }
                    }
                    // ...and check if the data storage holds all the data for them.
                    for (i, j) in b_tags {
                        if self
                            .storage
                            .get_b(i, j)
                            .await
                            .map_err(OperonError::Storage)?
                            .is_none()
                        {
                            info!("Data storage does not hold `B_{i},{j}`.");
                            return Ok(false);
                        }
                    }
                }
                '_c: {
                    // Pull the "done" gamma jobs from the metadata storage...
                    let client = self.meta_pool.get().await.map_err(meta_storage_error)?;
                    let conn = MetaStorageConnection {
                        client: &client,
                        schema: &self.meta_schema,
                    };
                    let Some(gamma_jobs) = tickets_psql::get_all_done::<_, GammaTicket>(conn)
                        .await?
                        .iter()
                        .map(|t| {
                            t.resolve().and_then(|job| {
                                if let job::Job::Gamma { i } = job {
                                    Some((i,))
                                } else {
                                    None
                                }
                            })
                        })
                        .collect::<Option<Vec<_>>>()
                    else {
                        info!("Some `gamma` tickets are corrupt in the metadata storage.");
                        return Ok(false);
                    };
                    // ...and map them with the dimensions they spawned...
                    let mut c_tags = vec![];
                    for (i,) in gamma_jobs {
                        let Some(k_ub) = facts_psql::get_resolution(
                            conn,
                            &masked_dimension::ResolutionRequest::K(i),
                        )
                        .await?
                        .and_then(|r| {
                            if let masked_dimension::Resolution::K(k_ub, _) = r {
                                Some(k_ub)
                            } else {
                                None
                            }
                        }) else {
                            info!(
                                "No `k` resolution found for `gamma_{i}` in the metadata storage."
                            );
                            return Ok(false);
                        };
                        for k in 0..k_ub {
                            c_tags.push((i, k));
                        }
                    }
                    // ...and check if the data storage holds all the data for them.
                    for (i, k) in c_tags {
                        if self
                            .storage
                            .get_c(i, k)
                            .await
                            .map_err(OperonError::Storage)?
                            .is_none()
                        {
                            info!("Data storage does not hold `G_{i},{k}`.");
                            return Ok(false);
                        }
                    }
                }
                '_d: {
                    // Pull the "done" delta jobs from the metadata storage...
                    let client = self.meta_pool.get().await.map_err(meta_storage_error)?;
                    let conn = MetaStorageConnection {
                        client: &client,
                        schema: &self.meta_schema,
                    };
                    let Some(delta_jobs) = tickets_psql::get_all_done::<_, DeltaTicket>(conn)
                        .await?
                        .iter()
                        .map(|t| {
                            t.resolve().and_then(|job| {
                                if let job::Job::Delta { i, j, k } = job {
                                    Some((i, j, k))
                                } else {
                                    None
                                }
                            })
                        })
                        .collect::<Option<Vec<_>>>()
                    else {
                        info!("Some `delta` tickets are corrupt in the metadata storage.");
                        return Ok(false);
                    };
                    // ...and check if the data storage holds all the data for them.
                    for (i, j, k) in delta_jobs {
                        if self
                            .storage
                            .get_d(i, j, k)
                            .await
                            .map_err(OperonError::Storage)?
                            .is_none()
                        {
                            info!("Data storage does not hold `D_{i},{j},{k}`.");
                            return Ok(false);
                        }
                    }
                }
                '_e: {
                    // Pull the "done" epsilon jobs from the metadata storage...
                    let client = self.meta_pool.get().await.map_err(meta_storage_error)?;
                    let conn = MetaStorageConnection {
                        client: &client,
                        schema: &self.meta_schema,
                    };
                    let Some(epsilon_jobs) = tickets_psql::get_all_done::<_, EpsilonTicket>(conn)
                        .await?
                        .iter()
                        .map(|t| {
                            t.resolve().and_then(|job| {
                                if let job::Job::Epsilon { i, k } = job {
                                    Some((i, k))
                                } else {
                                    None
                                }
                            })
                        })
                        .collect::<Option<Vec<_>>>()
                    else {
                        info!("Some `epsilon` tickets are corrupt in the metadata storage.");
                        return Ok(false);
                    };
                    // ...and check if the data storage holds all the data for them.
                    for (i, k) in epsilon_jobs {
                        if self
                            .storage
                            .get_e(i, k)
                            .await
                            .map_err(OperonError::Storage)?
                            .is_none()
                        {
                            info!("Data storage does not hold `E_{i},{k}`.");
                            return Ok(false);
                        }
                    }
                }
                '_f: {
                    // Pull the "done" zeta jobs from the metadata storage...
                    let client = self.meta_pool.get().await.map_err(meta_storage_error)?;
                    let conn = MetaStorageConnection {
                        client: &client,
                        schema: &self.meta_schema,
                    };
                    let Some(zeta_jobs) = tickets_psql::get_all_done::<_, ZetaTicket>(conn)
                        .await?
                        .iter()
                        .map(|t| {
                            t.resolve().and_then(|job| {
                                if let job::Job::Zeta { i } = job {
                                    Some((i,))
                                } else {
                                    None
                                }
                            })
                        })
                        .collect::<Option<Vec<_>>>()
                    else {
                        info!("Some `zeta` tickets are corrupt in the metadata storage.");
                        return Ok(false);
                    };
                    // ...and check if the data storage holds all the data for them.
                    for (i,) in zeta_jobs {
                        if self
                            .storage
                            .get_f(i)
                            .await
                            .map_err(OperonError::Storage)?
                            .is_none()
                        {
                            info!("Data storage does not hold `F_{i}`.");
                            return Ok(false);
                        }
                    }
                }
                Ok(true)
            }

            async fn update_ui_all<Cl>(&self, conn: MetaStorageConnection<'_, Cl>) -> Result<()>
            where
                Cl: MetaClient,
            {
                // Update the UI state with the current status of all individual schedulers.
                let beta_counts = tickets_psql::get_beta_status(conn).await?;
                let beta_state = if beta_counts.1 + beta_counts.2 == 0 {
                    RunningState::Finished
                } else {
                    RunningState::Running
                };
                let gamma_counts = tickets_psql::get_gamma_status(conn).await?;
                let gamma_state = if gamma_counts.1 + gamma_counts.2 == 0 {
                    RunningState::Finished
                } else {
                    RunningState::Running
                };
                let delta_counts = tickets_psql::get_delta_status(conn).await?;
                let delta_state = if delta_counts.1 + delta_counts.2 == 0 {
                    RunningState::Finished
                } else {
                    RunningState::Running
                };
                let epsilon_counts = tickets_psql::get_epsilon_status(conn).await?;
                let epsilon_state = if epsilon_counts.1 + epsilon_counts.2 == 0 {
                    RunningState::Finished
                } else {
                    RunningState::Running
                };
                let zeta_counts = tickets_psql::get_zeta_status(conn).await?;
                let zeta_state = if zeta_counts.1 + zeta_counts.2 == 0 {
                    RunningState::Finished
                } else {
                    RunningState::Running
                };

                ui::update_ui_state(
                    &self.ui_state.clone(),
                    ui::UiStateUpdate::Beta((
                        beta_counts.0,
                        beta_counts.1,
                        beta_counts.2,
                        beta_state,
                        false,
                    )),
                )
                .await;
                ui::update_ui_state(
                    &self.ui_state.clone(),
                    ui::UiStateUpdate::Gamma((
                        gamma_counts.0,
                        gamma_counts.1,
                        gamma_counts.2,
                        gamma_state,
                        false,
                    )),
                )
                .await;
                ui::update_ui_state(
                    &self.ui_state.clone(),
                    ui::UiStateUpdate::Delta((
                        delta_counts.0,
                        delta_counts.1,
                        delta_counts.2,
                        delta_state,
                        false,
                    )),
                )
                .await;
                ui::update_ui_state(
                    &self.ui_state.clone(),
                    ui::UiStateUpdate::Epsilon((
                        epsilon_counts.0,
                        epsilon_counts.1,
                        epsilon_counts.2,
                        epsilon_state,
                        false,
                    )),
                )
                .await;
                ui::update_ui_state(
                    &self.ui_state.clone(),
                    ui::UiStateUpdate::Zeta((
                        zeta_counts.0,
                        zeta_counts.1,
                        zeta_counts.2,
                        zeta_state,
                        false,
                    )),
                )
                .await;
                Ok(())
            }

            async fn run(mut self, primary_ub: usize, run_mode: RunMode) -> Result<()> {
                // Set up the initial storage setup and initial tickets for the individual schedulers.
                let initial_data = match run_mode {
                    RunMode::Clean => {
                        // Wipe the data storage clean.
                        self.storage.clear().await.map_err(OperonError::Storage)?;

                        // Wipe the metadata storage clean.
                        {
                            let mut client =
                                self.meta_pool.get().await.map_err(meta_storage_error)?;
                            let tx = client.transaction().await.map_err(meta_storage_error)?;
                            let conn = MetaStorageConnection {
                                client: &tx,
                                schema: &self.meta_schema,
                            };
                            facts_psql::clear(conn).await?;
                            facts_psql::put_resolution(
                                conn,
                                &masked_dimension::Resolution::I(primary_ub),
                            )
                            .await?;
                            tickets_psql::clear(conn).await?;
                            tickets_psql::put_default_tickets(conn).await?;
                            footprint_psql::clear(conn).await?;
                            tx.commit().await.map_err(meta_storage_error)?;
                        }

                        // No initial tickets.
                        (vec![], vec![], vec![], vec![], vec![])
                    }
                    RunMode::Rebuild => {
                        // * We *trust* the following data to be correct:
                        //   - The data storage,
                        //   - All dimension resolutions,
                        //   - All done tickets.
                        // * What we need to do:
                        //   - Pull the done tickets and potentially their associated resolutions,
                        //   - couple them into internal events `JobSuccess(job, resolution)`,
                        //   - wipe the metadata storage once that's done,
                        //   - roll the internal events out manually,
                        //   - and finally feed the resulting queued tickets to the individual schedulers.

                        // Clear the data storage's footprint. (The metadata storage will be cleared later.)
                        self.storage
                            .clear_footprint()
                            .await
                            .map_err(OperonError::Storage)?;
                        // Pull the done tickets from the metadata storage.
                        let mut client = self.meta_pool.get().await.map_err(meta_storage_error)?;
                        let tx = client.transaction().await.map_err(meta_storage_error)?;
                        let conn = MetaStorageConnection {
                            client: &tx,
                            schema: &self.meta_schema,
                        };
                        let beta_events = {
                            let mut beta_events = vec![];
                            let beta_tickets =
                                tickets_psql::get_all_done::<_, BetaTicket>(conn).await?;
                            for ticket in beta_tickets {
                                let job::Job::Beta { i } = ticket.resolve().ok_or_else(|| {
                                    scheduler_error_str("Failed to resolve a beta ticket")
                                })?
                                else {
                                    return Err(scheduler_error_str("Expected a beta job"));
                                };
                                let resolution = facts_psql::get_resolution(
                                    conn,
                                    &masked_dimension::ResolutionRequest::J(i),
                                )
                                .await?
                                .ok_or_else(|| {
                                    meta_storage_error_str(format!(
                                        "No resolution found for `J_{i}`"
                                    ))
                                })?;
                                beta_events.push(InternalEvent::JobSuccess(
                                    job::Job::Beta { i },
                                    Some(resolution),
                                ));
                            }
                            beta_events
                        };
                        let gamma_events = {
                            let mut gamma_events = vec![];
                            let gamma_tickets =
                                tickets_psql::get_all_done::<_, GammaTicket>(conn).await?;
                            for ticket in gamma_tickets {
                                let job::Job::Gamma { i } = ticket.resolve().ok_or_else(|| {
                                    scheduler_error_str("Failed to resolve a gamma ticket")
                                })?
                                else {
                                    return Err(scheduler_error_str("Expected a gamma job"));
                                };
                                let resolution = facts_psql::get_resolution(
                                    conn,
                                    &masked_dimension::ResolutionRequest::K(i),
                                )
                                .await?
                                .ok_or_else(|| {
                                    meta_storage_error_str(format!(
                                        "No resolution found for `K_{i}`"
                                    ))
                                })?;
                                gamma_events.push(InternalEvent::JobSuccess(
                                    job::Job::Gamma { i },
                                    Some(resolution),
                                ));
                            }
                            gamma_events
                        };
                        let delta_events = {
                            let mut delta_events = vec![];
                            let delta_tickets =
                                tickets_psql::get_all_done::<_, DeltaTicket>(conn).await?;
                            for ticket in delta_tickets {
                                let job::Job::Delta { i, j, k } =
                                    ticket.resolve().ok_or_else(|| {
                                        scheduler_error_str("Failed to resolve a delta ticket")
                                    })?
                                else {
                                    return Err(scheduler_error_str("Expected a delta job"));
                                };
                                delta_events.push(InternalEvent::JobSuccess(
                                    job::Job::Delta { i, j, k },
                                    None,
                                ));
                            }
                            delta_events
                        };
                        let epsilon_events = {
                            let mut epsilon_events = vec![];
                            let epsilon_tickets =
                                tickets_psql::get_all_done::<_, EpsilonTicket>(conn).await?;
                            for ticket in epsilon_tickets {
                                let job::Job::Epsilon { i, k } =
                                    ticket.resolve().ok_or_else(|| {
                                        scheduler_error_str("Failed to resolve an epsilon ticket")
                                    })?
                                else {
                                    return Err(scheduler_error_str("Expected an epsilon job"));
                                };
                                epsilon_events.push(InternalEvent::JobSuccess(
                                    job::Job::Epsilon { i, k },
                                    None,
                                ));
                            }
                            epsilon_events
                        };
                        let zeta_events = {
                            let mut zeta_events = vec![];
                            let zeta_tickets =
                                tickets_psql::get_all_done::<_, ZetaTicket>(conn).await?;
                            for ticket in zeta_tickets {
                                let job::Job::Zeta { i } = ticket.resolve().ok_or_else(|| {
                                    scheduler_error_str("Failed to resolve a zeta ticket")
                                })?
                                else {
                                    return Err(scheduler_error_str("Expected a zeta job"));
                                };
                                zeta_events
                                    .push(InternalEvent::JobSuccess(job::Job::Zeta { i }, None));
                            }
                            zeta_events
                        };

                        // Clear the metadata storage.
                        facts_psql::clear(conn).await?;
                        tickets_psql::clear(conn).await?;
                        footprint_psql::clear(conn).await?;

                        // Roll out the internal events manually.
                        // Initial setup:
                        '_a: {
                            tickets_psql::put_default_tickets(conn).await?;
                            let resolution = masked_dimension::Resolution::I(primary_ub);
                            facts_psql::put_resolution(conn, &resolution).await?;
                            tickets_psql::explode_beta(conn, &resolution).await?;
                            tickets_psql::explode_gamma(conn, &resolution).await?;
                            tickets_psql::explode_delta(conn, &resolution).await?;
                            tickets_psql::explode_epsilon(conn, &resolution).await?;
                            tickets_psql::explode_zeta(conn, &resolution).await?;
                            self.update_ui_all(conn).await?;
                        }
                        // beta events:
                        '_b: for event in beta_events {
                            let InternalEvent::JobSuccess(job::Job::Beta { i }, Some(resolution)) =
                                event
                            else {
                                return Err(scheduler_error_str(
                                    "Expected a beta job success event",
                                ));
                            };

                            // What we would do at a job success:
                            facts_psql::put_resolution(conn, &resolution).await?;
                            tickets_psql::mark_done_beta(conn, &i).await?;
                            // Roll them out to its dependencies:
                            tickets_psql::explode_delta(conn, &resolution).await?;
                            tickets_psql::raise_dep_delta(
                                conn,
                                &masked_dimension::I::One(i),
                                &masked_dimension::J::All(masked_dimension::I::One(i)),
                                &masked_dimension::K::All(masked_dimension::I::One(i)),
                            )
                            .await?;
                            tickets_psql::raise_dep_epsilon(
                                conn,
                                &masked_dimension::I::One(i),
                                &masked_dimension::K::All(masked_dimension::I::One(i)),
                            )
                            .await?;

                            self.update_ui_all(conn).await?;
                        }
                        debug!("Rebuilt `beta` jobs.");
                        '_c: for event in gamma_events {
                            let InternalEvent::JobSuccess(job::Job::Gamma { i }, Some(resolution)) =
                                event
                            else {
                                return Err(scheduler_error_str(
                                    "Expected a gamma job success event",
                                ));
                            };

                            // What we would do at a job success:
                            facts_psql::put_resolution(conn, &resolution).await?;
                            tickets_psql::mark_done_gamma(conn, &i).await?;
                            // Roll them out to its dependencies:
                            tickets_psql::explode_delta(conn, &resolution).await?;
                            tickets_psql::explode_epsilon(conn, &resolution).await?;
                            tickets_psql::raise_dep_delta(
                                conn,
                                &masked_dimension::I::One(i),
                                &masked_dimension::J::All(masked_dimension::I::One(i)),
                                &masked_dimension::K::All(masked_dimension::I::One(i)),
                            )
                            .await?;
                            tickets_psql::raise_dep_zeta(conn, &masked_dimension::I::One(i))
                                .await?;

                            self.update_ui_all(conn).await?;
                        }
                        debug!("Rebuilt `gamma` jobs.");
                        '_d: for event in delta_events {
                            let InternalEvent::JobSuccess(job::Job::Delta { i, j, k }, None) =
                                event
                            else {
                                return Err(scheduler_error_str(
                                    "Expected a delta job success event",
                                ));
                            };

                            // What we would do at a job success:
                            tickets_psql::mark_done_delta(conn, &i, &j, &k).await?;
                            // Roll them out to its dependencies:
                            tickets_psql::raise_dep_epsilon(
                                conn,
                                &masked_dimension::I::One(i),
                                &masked_dimension::K::One(k),
                            )
                            .await?;

                            self.update_ui_all(conn).await?;
                        }
                        debug!("Rebuilt `delta` jobs.");
                        '_e: for event in epsilon_events {
                            let InternalEvent::JobSuccess(job::Job::Epsilon { i, k }, None) = event
                            else {
                                return Err(scheduler_error_str(
                                    "Expected an epsilon job success event",
                                ));
                            };

                            // What we would do at a job success:
                            tickets_psql::mark_done_epsilon(conn, &i, &k).await?;
                            // Roll them out to its dependencies:
                            tickets_psql::raise_dep_zeta(conn, &masked_dimension::I::One(i))
                                .await?;

                            self.update_ui_all(conn).await?;
                        }
                        debug!("Rebuilt `epsilon` jobs.");
                        '_f: for event in zeta_events {
                            let InternalEvent::JobSuccess(job::Job::Zeta { i }, None) = event
                            else {
                                return Err(scheduler_error_str(
                                    "Expected a zeta job success event",
                                ));
                            };

                            // What we would do at a job success:
                            tickets_psql::mark_done_zeta(conn, &i).await?;

                            self.update_ui_all(conn).await?;
                        }
                        debug!("Rebuilt `zeta` jobs.");
                        tx.commit().await.map_err(meta_storage_error)?;
                        info!("Rebuild complete, starting the run.");

                        // Finally, pull the queued tickets from the metadata storage.
                        let client = self.meta_pool.get().await.map_err(meta_storage_error)?;
                        let conn = MetaStorageConnection {
                            client: &client,
                            schema: &self.meta_schema,
                        };
                        (
                            tickets_psql::get_all_queued::<_, BetaTicket>(conn).await?,
                            tickets_psql::get_all_queued::<_, GammaTicket>(conn).await?,
                            tickets_psql::get_all_queued::<_, DeltaTicket>(conn).await?,
                            tickets_psql::get_all_queued::<_, EpsilonTicket>(conn).await?,
                            tickets_psql::get_all_queued::<_, ZetaTicket>(conn).await?,
                        )
                    }
                    RunMode::Restore => {
                        // * The persistent storage is fully trusted.
                        // * Just pull the queued tickets, and have the individual schedulers'
                        //   initial `ready_to_run` set to them.
                        // * We need to clear the footprint only.

                        // Clear the data storage's footprint.
                        self.storage
                            .clear_footprint()
                            .await
                            .map_err(OperonError::Storage)?;
                        // Clear the metadata storage's footprint.
                        {
                            let mut client =
                                self.meta_pool.get().await.map_err(meta_storage_error)?;
                            let tx = client.transaction().await.map_err(meta_storage_error)?;
                            let conn = MetaStorageConnection {
                                client: &tx,
                                schema: &self.meta_schema,
                            };
                            footprint_psql::clear(conn).await?;
                            tx.commit().await.map_err(meta_storage_error)?;
                        }

                        // Pull the queued tickets from the metadata storage, and set the initial data.
                        let client = self.meta_pool.get().await.map_err(meta_storage_error)?;
                        let conn = MetaStorageConnection {
                            client: &client,
                            schema: &self.meta_schema,
                        };
                        (
                            tickets_psql::get_all_queued::<_, BetaTicket>(conn).await?,
                            tickets_psql::get_all_queued::<_, GammaTicket>(conn).await?,
                            tickets_psql::get_all_queued::<_, DeltaTicket>(conn).await?,
                            tickets_psql::get_all_queued::<_, EpsilonTicket>(conn).await?,
                            tickets_psql::get_all_queued::<_, ZetaTicket>(conn).await?,
                        )
                    }
                };
                // Create the mpsc channels.
                let (peer_tx_beta, peer_rx_beta) =
                    ::tokio::sync::mpsc::channel::<PeerEvent>(INTERNAL_CHANNEL_SIZE);
                let (peer_tx_gamma, peer_rx_gamma) =
                    ::tokio::sync::mpsc::channel::<PeerEvent>(INTERNAL_CHANNEL_SIZE);
                let (peer_tx_delta, peer_rx_delta) =
                    ::tokio::sync::mpsc::channel::<PeerEvent>(INTERNAL_CHANNEL_SIZE);
                let (peer_tx_epsilon, peer_rx_epsilon) =
                    ::tokio::sync::mpsc::channel::<PeerEvent>(INTERNAL_CHANNEL_SIZE);
                let (peer_tx_zeta, peer_rx_zeta) =
                    ::tokio::sync::mpsc::channel::<PeerEvent>(INTERNAL_CHANNEL_SIZE);

                let _ = self.peer_txs.insert(PeerEventSenders {
                    to_beta: PeerEventSender::Up(peer_tx_beta),
                    to_gamma: PeerEventSender::Up(peer_tx_gamma),
                    to_delta: PeerEventSender::Up(peer_tx_delta),
                    to_epsilon: PeerEventSender::Up(peer_tx_epsilon),
                    to_zeta: PeerEventSender::Up(peer_tx_zeta),
                });

                // Pool sizes are parsed from the config TOML; 1 if not specified.
                let meta_pool_cloned = self.meta_pool.clone();
                let meta_schema_cloned = self.meta_schema.clone();
                let peer_txs_cloned = self.peer_txs.clone().unwrap();
                let ctrl_rx_cloned = self.ctrl_rx.clone();
                let storage_cloned = self.storage.clone();
                let service_cloned = self.service.clone();
                let ui_state_cloned = self.ui_state.clone();
                self.handles.spawn(async move {
                    IndividualScheduler::<BetaTicket>::new(
                        meta_pool_cloned,
                        meta_schema_cloned,
                        8,
                        ui_state_cloned,
                        peer_txs_cloned,
                        peer_rx_beta,
                        ctrl_rx_cloned,
                    )
                    .run(storage_cloned, service_cloned, initial_data.0)
                    .await
                });

                let meta_pool_cloned = self.meta_pool.clone();
                let meta_schema_cloned = self.meta_schema.clone();
                let peer_txs_cloned = self.peer_txs.clone().unwrap();
                let ctrl_rx_cloned = self.ctrl_rx.clone();
                let storage_cloned = self.storage.clone();
                let service_cloned = self.service.clone();
                let ui_state_cloned = self.ui_state.clone();
                self.handles.spawn(async move {
                    IndividualScheduler::<GammaTicket>::new(
                        meta_pool_cloned,
                        meta_schema_cloned,
                        8,
                        ui_state_cloned,
                        peer_txs_cloned,
                        peer_rx_gamma,
                        ctrl_rx_cloned,
                    )
                    .run(storage_cloned, service_cloned, initial_data.1)
                    .await
                });

                let meta_pool_cloned = self.meta_pool.clone();
                let meta_schema_cloned = self.meta_schema.clone();
                let peer_senders_cloned = self.peer_txs.clone().unwrap();
                let ctrl_rx_cloned = self.ctrl_rx.clone();
                let storage_cloned = self.storage.clone();
                let service_cloned = self.service.clone();
                let ui_state_cloned = self.ui_state.clone();
                self.handles.spawn(async move {
                    IndividualScheduler::<DeltaTicket>::new(
                        meta_pool_cloned,
                        meta_schema_cloned,
                        4,
                        ui_state_cloned,
                        peer_senders_cloned,
                        peer_rx_delta,
                        ctrl_rx_cloned,
                    )
                    .run(storage_cloned, service_cloned, initial_data.2)
                    .await
                });

                let meta_pool_cloned = self.meta_pool.clone();
                let meta_schema_cloned = self.meta_schema.clone();
                let peer_senders_cloned = self.peer_txs.clone().unwrap();
                let ctrl_rx_cloned = self.ctrl_rx.clone();
                let storage_cloned = self.storage.clone();
                let service_cloned = self.service.clone();
                let ui_state_cloned = self.ui_state.clone();
                self.handles.spawn(async move {
                    IndividualScheduler::<EpsilonTicket>::new(
                        meta_pool_cloned,
                        meta_schema_cloned,
                        4,
                        ui_state_cloned,
                        peer_senders_cloned,
                        peer_rx_epsilon,
                        ctrl_rx_cloned,
                    )
                    .run(storage_cloned, service_cloned, initial_data.3)
                    .await
                });

                let meta_pool_cloned = self.meta_pool.clone();
                let meta_schema_cloned = self.meta_schema.clone();
                let peer_senders_cloned = self.peer_txs.clone().unwrap();
                let ctrl_rx_cloned = self.ctrl_rx.clone();
                let storage_cloned = self.storage.clone();
                let service_cloned = self.service.clone();
                let ui_state_cloned = self.ui_state.clone();
                self.handles.spawn(async move {
                    IndividualScheduler::<ZetaTicket>::new(
                        meta_pool_cloned,
                        meta_schema_cloned,
                        1,
                        ui_state_cloned,
                        peer_senders_cloned,
                        peer_rx_zeta,
                        ctrl_rx_cloned,
                    )
                    .run(storage_cloned, service_cloned, initial_data.4)
                    .await
                });

                match run_mode {
                    // If the run is `Clean`, we send the initial resolution to all individual schedulers.
                    RunMode::Clean => {
                        let resolution = {
                            let client = self.meta_pool.get().await.map_err(meta_storage_error)?;
                            // Read-only, so no transaction needed.
                            let conn = MetaStorageConnection {
                                client: &client,
                                schema: &self.meta_schema,
                            };
                            facts_psql::get_resolution(
                                conn,
                                &masked_dimension::ResolutionRequest::I,
                            )
                            .await?
                            .ok_or_else(|| {
                                meta_storage_error_str("No initial resolution found".to_string())
                            })?
                        };
                        let masked_dimension::Resolution::I(resolved_i) = resolution else {
                            return Err(meta_storage_error_str(
                                "Expected initial resolution to be I".to_string(),
                            ));
                        };
                        // Send the initial resolution, then drop this scheduler's peer_txs.
                        {
                            let peer_txs = self.peer_txs.take().unwrap();
                            peer_txs
                                .to_beta
                                .send(PeerEvent::Resolution(masked_dimension::Resolution::I(
                                    resolved_i,
                                )))
                                .await?;
                            peer_txs
                                .to_gamma
                                .send(PeerEvent::Resolution(masked_dimension::Resolution::I(
                                    resolved_i,
                                )))
                                .await?;
                            peer_txs
                                .to_delta
                                .send(PeerEvent::Resolution(masked_dimension::Resolution::I(
                                    resolved_i,
                                )))
                                .await?;
                            peer_txs
                                .to_epsilon
                                .send(PeerEvent::Resolution(masked_dimension::Resolution::I(
                                    resolved_i,
                                )))
                                .await?;
                            peer_txs
                                .to_zeta
                                .send(PeerEvent::Resolution(masked_dimension::Resolution::I(
                                    resolved_i,
                                )))
                                .await?;
                        }
                    }
                    // Otherwise, we just drop `peer_txs`.
                    _ => {
                        // No initial resolution is sent, as the individual schedulers
                        // will pull the queued tickets from the metadata storage.
                        self.peer_txs.take();
                    }
                }

                let mut returned_states = vec![];
                while let Some(res) = self.handles.join_next().await {
                    // Bubble up JoinError and push the resulting state.
                    returned_states.push(res.map_err(scheduler_error)?);
                }
                // Here, we have the following possible states:
                // 1. All jobs finished successfully.
                // 2. All jobs are `Finished` or `Stopped`, and the control signal is a `GracefulStop` event.
                // 3-1. All jobs are `Finished` or `Stopped`, and the control signal is an `Abort` event.
                // 3-2. Some jobs returned an `Error`.
                // On `1`, we set the footprint to "F@{now}".
                // On `2`, we set the footprint to "S@{now}".
                // On `3`, we don't set the footprint.
                let footprint = {
                    let now = ::chrono::Local::now();
                    if returned_states.iter().all(|&s| s == RunningState::Finished) {
                        Some(format!("F@{now}"))
                    } else if returned_states
                        .iter()
                        .all(|&s| s == RunningState::Stopped || s == RunningState::Finished)
                        && self.ctrl_rx.borrow().clone() == ControlEvent::GracefulStop
                    {
                        Some(format!("S@{now}"))
                    } else {
                        None
                    }
                };
                if let Some(footprint) = footprint {
                    // Write the footprint to the data storage...
                    self.storage
                        .put_footprint(&footprint)
                        .await
                        .map_err(OperonError::Storage)?;
                    // ...and to the metadata storage.
                    let mut client = self.meta_pool.get().await.map_err(meta_storage_error)?;
                    let tx = client.transaction().await.map_err(meta_storage_error)?;
                    let conn = MetaStorageConnection {
                        client: &tx,
                        schema: &self.meta_schema,
                    };
                    footprint_psql::put_footprint(conn, "global".to_string(), footprint).await?;
                    tx.commit().await.map_err(meta_storage_error)?;
                }

                info!("All jobs closed.");

                Ok(())
            }
        }
    }

    /// Terminal UI module with `ratatui`, `crossterm`.
    /// This module provides a simple terminal UI for Operon.
    pub mod ui {
        #![allow(unreachable_patterns)]

        use super::scheduler::ControlEvent;
        use super::*;
        use ::clap::{Parser, Subcommand};
        use ::crossterm::{
            event::{KeyCode, KeyEvent, KeyModifiers, MouseEventKind},
            execute,
            terminal::{
                EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
            },
        };
        use ::ratatui::{prelude::*, widgets::*};
        use ::std::sync::Arc;
        use ::tokio::sync::RwLock;

        #[allow(dead_code)]
        #[derive(Debug, Clone)]
        pub struct LogRecord {
            timestamp: ::chrono::DateTime<::chrono::offset::Local>,
            level: ::log::Level,
            target: String,
            file: Option<String>,
            module_path: Option<String>,
            line: Option<u32>,
            msg: String,
        }
        impl From<&::log::Record<'_>> for LogRecord {
            fn from(record: &::log::Record<'_>) -> Self {
                LogRecord {
                    timestamp: ::chrono::Local::now(),
                    level: record.level(),
                    target: record.target().to_string(),
                    file: record.file().map(|s| s.to_string()),
                    module_path: record.module_path().map(|s| s.to_string()),
                    line: record.line(),
                    msg: record.args().to_string(),
                }
            }
        }
        impl LogRecord {
            pub fn format(&self, width: u16) -> Vec<Line<'_>> {
                let timestamp = self.timestamp.format("%y-%m-%d %H:%M:%S").to_string();
                let level_colour = match self.level {
                    log::Level::Error => ::ratatui::style::Style::new().red(),
                    log::Level::Warn => ::ratatui::style::Style::new().yellow(),
                    log::Level::Info => ::ratatui::style::Style::new().green(),
                    log::Level::Debug => ::ratatui::style::Style::new().cyan(),
                    log::Level::Trace => ::ratatui::style::Style::new().white(),
                };
                // Colour messages that echo shell input.
                let msg_colour = if self.msg.starts_with("$ ") {
                    match self.level {
                        log::Level::Error => ::ratatui::style::Style::new().light_red(),
                        log::Level::Info => ::ratatui::style::Style::new().light_green(),
                        _ => ::ratatui::style::Style::new(),
                    }
                } else {
                    ::ratatui::style::Style::new()
                };
                let level_label = self.level.as_str();
                let leading_prefix = format!("{timestamp} {level_label:>5}│ ");
                let wrap_prefix = "                    ...│ ".to_string();

                let mut lines: Vec<Line<'_>> = vec![];
                for (i, line) in self.msg.lines().enumerate() {
                    let initial_prefix = if i == 0 {
                        leading_prefix.clone()
                    } else {
                        format!("{:>22} │ ", i + 1)
                    };

                    let options = ::textwrap::Options::new(width as usize)
                        .initial_indent(&initial_prefix)
                        .subsequent_indent(&wrap_prefix);
                    let wrapped = ::textwrap::wrap(line, options)
                        .iter()
                        .enumerate()
                        .map(|(l, s)| {
                            let chars = s.chars();
                            Line::from(match l {
                                0 => vec![
                                    Span::raw(chars.clone().take(18).collect::<String>()),
                                    Span::styled(
                                        chars.clone().skip(18).take(6).collect::<String>(),
                                        level_colour,
                                    ),
                                    Span::raw(chars.clone().skip(24).take(2).collect::<String>()),
                                    Span::styled(chars.skip(26).collect::<String>(), msg_colour),
                                ],
                                _ => vec![
                                    Span::raw(chars.clone().take(18).collect::<String>()),
                                    Span::styled(
                                        chars.clone().skip(18).take(6).collect::<String>(),
                                        level_colour,
                                    ),
                                    Span::styled(chars.skip(24).collect::<String>(), msg_colour),
                                ],
                            })
                        })
                        .collect::<Vec<_>>();
                    lines.extend(wrapped);
                }
                lines
            }
            pub fn dump_format(&self) -> String {
                format!(
                    "{},{},{},{},{},{},\"{}\"",
                    self.timestamp.format("%Y-%m-%d %H:%M:%S%.f %:z"),
                    self.level,
                    self.target,
                    self.file.as_deref().unwrap_or(""),
                    self.module_path.as_deref().unwrap_or(""),
                    self.line.unwrap_or(0),
                    self.msg.replace("\"", "\"\"")
                )
            }
        }

        #[derive(Debug)]
        pub struct UiLogger {
            sender: ::tokio::sync::broadcast::Sender<LogRecord>,
        }
        impl UiLogger {
            pub fn new(sender: ::tokio::sync::broadcast::Sender<LogRecord>) -> Self {
                Self { sender }
            }
            pub fn blacklisted(&self, metadata: &::log::Metadata) -> bool {
                // Blacklist logs from external crates that come from inside Operon.
                // These are usually too verbose and not useful for the user.
                // These blacklists should be configurable by the user.
                if metadata.target().starts_with("tokio_postgres")
                    && metadata.level() >= ::log::Level::Info
                {
                    return true;
                }
                if metadata.target().starts_with("mio::poll")
                    && metadata.level() >= ::log::Level::Trace
                {
                    return true;
                }
                false
            }
        }
        impl ::log::Log for UiLogger {
            fn enabled(&self, metadata: &::log::Metadata) -> bool {
                if self.blacklisted(metadata) {
                    return false;
                }
                metadata.level() <= LOG_LEVEL
            }
            fn log(&self, record: &::log::Record) {
                // Dump to the log file, regardless of the log level, unless it's blacklisted.
                if LOG_DUMP && !self.blacklisted(record.metadata()) {
                    use ::std::io::Write;
                    if !::std::path::Path::new(LOG_DUMP_DIR).exists() {
                        ::std::fs::create_dir_all(LOG_DUMP_DIR).unwrap();
                    }
                    let log_file = match ::std::fs::OpenOptions::new()
                        .append(true)
                        .create_new(true)
                        .open(::std::path::PathBuf::from(LOG_DUMP_DIR).join("operon.csv"))
                    {
                        Ok(file) => {
                            let mut writer = ::std::io::BufWriter::new(file);
                            let _ =
                                writeln!(writer, "timestamp,level,target,file,module,line,message");
                            writer.into_inner().unwrap()
                        }
                        Err(ref e) if e.kind() == ::std::io::ErrorKind::AlreadyExists => {
                            ::std::fs::OpenOptions::new()
                                .append(true)
                                .open(::std::path::PathBuf::from(LOG_DUMP_DIR).join("operon.csv"))
                                .unwrap()
                        }
                        Err(_) => {
                            panic!("Couldn't open log file")
                        }
                    };
                    let mut writer = ::std::io::BufWriter::new(log_file);
                    let record = LogRecord::from(record);
                    let _ = writeln!(writer, "{}", record.dump_format());
                }

                // Then send the record to the UI logger.
                // This fails when the UI closes, which is fine.
                if self.enabled(record.metadata()) {
                    let _ = self.sender.send(record.into());
                }
            }
            fn flush(&self) {
                // No-op
            }
        }

        #[derive(Debug, Clone)]
        pub struct LogBuffer {
            records: ::std::collections::VecDeque<LogRecord>,
            capacity: usize,
        }
        impl Default for LogBuffer {
            fn default() -> Self {
                Self::new()
            }
        }
        impl LogBuffer {
            pub fn new() -> Self {
                Self {
                    records: ::std::collections::VecDeque::with_capacity(LOG_BUFFER_SIZE),
                    capacity: LOG_BUFFER_SIZE,
                }
            }
            pub fn push(&mut self, record: LogRecord) {
                if self.records.len() >= self.capacity {
                    self.records.pop_front();
                }
                self.records.push_back(record);
            }
            pub fn clear(&mut self) {
                self.records.clear();
            }
            pub fn to_lines(&self, width: u16) -> Vec<Line<'_>> {
                self.records.iter().flat_map(|r| r.format(width)).collect()
            }
            /// Display the bottom `height` lines, skipping `cursor` lines.
            /// If `cursor + height` exceeds the number of lines, `cursor` will be clamped down.
            pub fn to_text(&self, width: u16, height: u16, cursor: usize) -> (Text<'_>, usize) {
                let lines = self.to_lines(width);
                let cursor = cursor.min(lines.len().saturating_sub(height as usize));
                let num_buffer_lines = height.saturating_sub((lines.len() - cursor) as u16);
                let visible_lines = lines
                    .iter()
                    .rev()
                    .skip(cursor)
                    .take(height as usize)
                    .cloned()
                    .chain(::std::iter::repeat_n(
                        Line::from(""),
                        num_buffer_lines as usize,
                    ))
                    .rev()
                    .collect::<Vec<_>>();
                (Text::from(visible_lines), cursor)
            }
        }

        #[derive(Parser, Debug, Clone)]
        #[clap(name = "operon")]
        #[command(disable_help_flag = true, disable_help_subcommand = true)]
        pub struct PromptCommand {
            #[clap(subcommand)]
            pub action: Action,
        }
        #[derive(Subcommand, Debug, Clone)]
        #[clap(rename_all = "snake_case")]
        pub enum Action {
            /// Start a new run using the best available restoration.
            Run {
                /// Start a fresh run, ignoring any existing data.
                #[clap(short, long)]
                fresh: bool,
                /// Rebuild the run from trusted data before starting.
                #[clap(short, long)]
                rebuild: bool,
            },
            /// Check the consistency of the data from the last run.
            Check,
            /// Exit the UI.
            Exit,
            /// Clear the log buffer.
            Clear,
            /// Quit Operon.
            Quit {
                /// Force quit, ignoring running jobs.
                #[clap(short, long)]
                force: bool,
                /// Don't exit the UI.
                #[clap(short, long = "no-exit")]
                no_exit: bool,
            },
            /// Pause jobs.
            Pause {
                /// Target job(s) to pause.
                /// If not specified, pauses all jobs.
                #[clap()]
                targets: Vec<JobType>,
                /// Whether to cascade down the pause command
                /// to all dependent jobs.
                #[clap(short, long)]
                cascade: bool,
            },
            /// Resume jobs.
            Resume {
                /// Target job(s) to resume.
                /// If not specified, resumes all jobs.
                #[clap()]
                targets: Vec<JobType>,
            },
            /// Print help.
            Help,
        }
        const HELP_TEXT: &str = r#"Operon TUI.
Navigation keys:
    ^C                  Clear input.
    ^D                  Exit.
    ^L                  Clear logs.
    ^Up, ^Down          Scroll logs 1 line.
    Up, Down            Scroll logs 5 lines.
    PgUp, PgDn          Scroll logs 20 lines.
    Esc                 Show most recent logs.

Commands:
    run [OPTIONS]       Start a new run using the best available restoration (unless overridden by options).
        -f, --fresh         Start a fresh run, ignoring any existing data. Takes precedence over `rebuild`.
        -r, --rebuild       Rebuild the run from trusted data before starting.
    check               Check the consistency of the data from the last run.
    exit                Exit the UI.
    clear               Clear the log buffer.
    quit [OPTIONS]      Stop all jobs and exit the UI. Defaults to graceful shutdown.
        -f, --force         Force quit.
        -n, --no-exit       Don't exit the UI.
    pause [OPTIONS] [<JOB_TYPE>[ ...]]
                        Pause executing new jobs.
        -c, --cascade       Cascade the pause command to dependent jobs.
    resume [<JOB_TYPE>[ ...]]
                        Resume paused jobs.
    help                Print this help message."#;

        #[derive(Default, Debug, Clone)]
        pub struct ShellPrompt {
            input: String,
        }
        impl ShellPrompt {
            fn on_key(&mut self, key: KeyEvent) -> Option<PromptCommand> {
                match key {
                    KeyEvent {
                        code: KeyCode::Char(c),
                        modifiers: crossterm::event::KeyModifiers::CONTROL,
                        ..
                    } => match c {
                        'c' => {
                            self.input.clear();
                            None
                        }
                        'd' => {
                            if self.input.is_empty() {
                                Some(PromptCommand {
                                    action: Action::Exit,
                                })
                            } else {
                                None
                            }
                        }
                        'l' => {
                            if self.input.is_empty() {
                                Some(PromptCommand {
                                    action: Action::Clear,
                                })
                            } else {
                                None
                            }
                        }
                        _ => None,
                    },
                    KeyEvent {
                        code: KeyCode::Char(c),
                        ..
                    } => {
                        self.input.push(c);
                        None
                    }
                    KeyEvent {
                        code: KeyCode::Backspace,
                        ..
                    } => {
                        self.input.pop();
                        None
                    }
                    KeyEvent {
                        code: KeyCode::Enter,
                        ..
                    } => {
                        let clap_input = String::from("operon ") + &self.input;
                        let args = clap_input.split_whitespace();
                        if args.clone().count() == 1 {
                            // No command entered, just return None
                            self.input.clear();
                            return None;
                        }
                        let command = match PromptCommand::try_parse_from(args) {
                            Ok(cmd) => {
                                info!("$ {}", self.input);
                                Some(cmd)
                            }
                            Err(_) => {
                                error!("$ {}", self.input);
                                None
                            }
                        };
                        self.input.clear();
                        command
                    }
                    _ => None,
                }
            }
            fn render(
                &self,
                frame: &mut ::ratatui::Frame,
                area: ::ratatui::layout::Rect,
                color: Color,
            ) {
                let line = Line::from(vec![
                    // This reads "operon@<package.name>$ ".
                    Span::styled("operon@ex1$ ", Style::new().fg(color)),
                    Span::raw(self.input.clone()),
                    Span::raw("█"),
                ])
                .left_aligned();
                frame.render_widget(line, area);
            }
        }

        pub type Progress = (i64, i64, i64, RunningState, bool);
        /// Minimal state that holds the information needed to render the UI.
        #[derive(Default, Debug, Clone)]
        pub struct UiState {
            // Done, queued, waiting, state, returned.
            beta: Progress,
            gamma: Progress,
            delta: Progress,
            epsilon: Progress,
            zeta: Progress,
            log_buffer: LogBuffer,
            /// Corresponds to how many bottom lines to skip
            cursor: usize,
            unread_logs: usize,
            shell: ShellPrompt,
            exit_on_finish: bool,
            /// Last sent control event.
            last_control_event: ControlEvent,
        }
        impl UiState {
            fn progress_iter(&self) -> impl Iterator<Item = Progress> {
                [self.beta, self.gamma, self.delta, self.epsilon, self.zeta].into_iter()
            }
            fn state_iter(&self) -> impl Iterator<Item = RunningState> {
                [
                    self.beta.3,
                    self.gamma.3,
                    self.delta.3,
                    self.epsilon.3,
                    self.zeta.3,
                ]
                .into_iter()
            }
            fn any_alive(&self) -> bool {
                self.progress_iter().any(|s| !s.4)
            }
        }

        pub enum UiStateUpdate {
            // Ticket progress updates.
            Beta(Progress),
            Gamma(Progress),
            Delta(Progress),
            Epsilon(Progress),
            Zeta(Progress),
            // New log record.
            NewLog(LogRecord, u16),
            SetCursor(usize),
            ExitOnFinish(bool),
            LastControlEvent(ControlEvent),
        }

        fn overall_state(ui_state: &UiState) -> RunningState {
            if ui_state.state_iter().any(|s| s == RunningState::Error) {
                RunningState::Error
            } else if ui_state.state_iter().all(|s| s == RunningState::Finished) {
                RunningState::Finished
            } else if ui_state
                .state_iter()
                .all(|s| s == RunningState::Paused || s == RunningState::Finished)
            {
                RunningState::Paused
            } else if ui_state
                .state_iter()
                .all(|s| s == RunningState::Stopped || s == RunningState::Finished)
            {
                RunningState::Stopped
            } else {
                RunningState::Running
            }
        }

        pub async fn update_ui_state(state: &Arc<RwLock<UiState>>, update: UiStateUpdate) {
            let mut guard = state.write().await;
            match update {
                UiStateUpdate::Beta(progress) => {
                    guard.beta = progress;
                }
                UiStateUpdate::Gamma(progress) => {
                    guard.gamma = progress;
                }
                UiStateUpdate::Delta(progress) => {
                    guard.delta = progress;
                }
                UiStateUpdate::Epsilon(progress) => {
                    guard.epsilon = progress;
                }
                UiStateUpdate::Zeta(progress) => {
                    guard.zeta = progress;
                }
                UiStateUpdate::NewLog(record, width) => {
                    // Follow the cursor if it is not at 0
                    if guard.cursor != 0 {
                        guard.cursor = guard.cursor.saturating_add(record.format(width).len());
                        guard.unread_logs += 1;
                    }
                    guard.log_buffer.push(record);
                }
                UiStateUpdate::SetCursor(cursor) => {
                    guard.cursor = cursor;
                    if cursor == 0 {
                        guard.unread_logs = 0;
                    }
                }
                UiStateUpdate::ExitOnFinish(yes) => {
                    guard.exit_on_finish = yes;
                }
                UiStateUpdate::LastControlEvent(event) => {
                    guard.last_control_event = event;
                }
            }
        }

        /// Exposed entrypoint for drawing the UI.
        /// Put behind a `tokio::spawn`.
        pub async fn run_ui(
            state: Arc<RwLock<UiState>>,
            primary_ub: usize,
            mut log_rx: ::tokio::sync::broadcast::Receiver<LogRecord>,
            ctrl_tx: ::tokio::sync::watch::Sender<ControlEvent>,
            rec_rx: ::tokio::sync::watch::Receiver<super::scheduler::RecoveryState>,
        ) -> super::Result<()> {
            use ::futures::StreamExt;
            enable_raw_mode().map_err(ui_error)?;
            let mut stdout = ::std::io::stdout();
            execute!(stdout, EnterAlternateScreen).map_err(ui_error)?;
            let backend = CrosstermBackend::new(stdout);
            let mut terminal = Terminal::new(backend).map_err(ui_error)?;
            terminal.clear().map_err(ui_error)?;

            // Main loop for the UI.
            // Note: breaking this loop exits the UI, at least guard against `any_alive` before breaking.
            let mut events = ::crossterm::event::EventStream::new();
            loop {
                {
                    let guard = state.read().await;
                    if !guard.any_alive() && guard.exit_on_finish {
                        break;
                    }
                }
                tokio::select! {
                    evt = events.next() => {
                        let Some(evt) = evt else {
                            // Stream closed, exit the UI.
                            break;
                        };
                        let command = match evt.map_err(ui_error)? {
                            ::crossterm::event::Event::Key(KeyEvent {
                                code: KeyCode::Up,
                                modifiers: KeyModifiers::CONTROL,
                                ..
                            }) => {
                                let mut state = state.write().await;
                                state.cursor = state.cursor.saturating_add(1);
                                None
                            }
                            ::crossterm::event::Event::Key(KeyEvent {
                                code: KeyCode::Down,
                                modifiers: KeyModifiers::CONTROL,
                                ..
                            }) => {
                                let mut state = state.write().await;
                                state.cursor = state.cursor.saturating_sub(1);
                                if state.cursor == 0 {
                                    state.unread_logs = 0;
                                }
                                None
                            }
                            ::crossterm::event::Event::Key(KeyEvent {
                                code: KeyCode::Up, ..
                            }) => {
                                let mut state = state.write().await;
                                state.cursor = state.cursor.saturating_add(5);
                                None
                            }
                            ::crossterm::event::Event::Key(KeyEvent {
                                code: KeyCode::Down,
                                ..
                            }) => {
                                let mut state = state.write().await;
                                state.cursor = state.cursor.saturating_sub(5);
                                if state.cursor == 0 {
                                    state.unread_logs = 0;
                                }
                                None
                            }
                            ::crossterm::event::Event::Key(KeyEvent {
                                code: KeyCode::PageUp,
                                ..
                            }) => {
                                let mut state = state.write().await;
                                state.cursor = state.cursor.saturating_add(20);
                                None
                            }
                            ::crossterm::event::Event::Key(KeyEvent {
                                code: KeyCode::PageDown,
                                ..
                            }) => {
                                let mut state = state.write().await;
                                state.cursor = state.cursor.saturating_sub(20);
                                if state.cursor == 0 {
                                    state.unread_logs = 0;
                                }
                                None
                            }
                            ::crossterm::event::Event::Key(KeyEvent {
                                code: KeyCode::Esc, ..
                            }) => {
                                let mut state = state.write().await;
                                state.cursor = 0;
                                state.unread_logs = 0;
                                None
                            }
                            ::crossterm::event::Event::Key(key) => {
                                state.write().await.shell.on_key(key)
                            }
                            ::crossterm::event::Event::Mouse(me) => match me.kind {
                                MouseEventKind::ScrollUp => {
                                    let mut state = state.write().await;
                                    state.cursor = state.cursor.saturating_add(5);
                                    None
                                }
                                MouseEventKind::ScrollDown => {
                                    let mut state = state.write().await;
                                    state.cursor = state.cursor.saturating_sub(5);
                                    if state.cursor == 0 {
                                        state.unread_logs = 0;
                                    }
                                    None
                                }
                                _ => None,
                            },
                            _ => None,
                        };

                        // Fetch the recovery state.
                        let rec_state = *rec_rx.borrow();
                        if rec_state == scheduler::RecoveryState::Error {
                            error!("Error in scheduler startup, exiting UI.");
                            break;
                        }
                        // Lock and clone current UiState for command execution.
                        let exec_snapshot = {
                            let guard = state.read().await;
                            guard.clone()
                        };
                        let overall_state_snapshot = overall_state(&exec_snapshot);
                        // Execute the command if any.
                        if let Some(command) = command {
                            match command.action {
                                Action::Run { fresh, rebuild } => match exec_snapshot.last_control_event {
                                    ControlEvent::Start => match rec_state {
                                        scheduler::RecoveryState::Unknown => {
                                            warn!("Scheduler was not initialized yet.")
                                        }
                                        scheduler::RecoveryState::Fresh
                                        | scheduler::RecoveryState::Finished => {
                                            if rebuild {
                                                error!("Cannot rebuild.")
                                            } else {
                                                ctrl_tx
                                                    .send(ControlEvent::CleanRun { primary_ub })
                                                    .map_err(ui_error)?;
                                                update_ui_state(
                                                    &state,
                                                    UiStateUpdate::LastControlEvent(
                                                        ControlEvent::CleanRun { primary_ub },
                                                    ),
                                                )
                                                .await;
                                            }
                                        }
                                        scheduler::RecoveryState::AbortedUnchecked => {
                                            if rebuild {
                                                error!("Cannot rebuild before checking for consistency.")
                                            } else {
                                                ctrl_tx
                                                    .send(ControlEvent::CleanRun { primary_ub })
                                                    .map_err(ui_error)?;
                                                update_ui_state(
                                                    &state,
                                                    UiStateUpdate::LastControlEvent(
                                                        ControlEvent::CleanRun { primary_ub },
                                                    ),
                                                )
                                                .await;
                                            }
                                        }
                                        scheduler::RecoveryState::AbortedChecked => {
                                            if fresh {
                                                info!("Starting a fresh run, ignoring previous data.");
                                                ctrl_tx
                                                    .send(ControlEvent::CleanRun { primary_ub })
                                                    .map_err(ui_error)?;
                                                update_ui_state(
                                                    &state,
                                                    UiStateUpdate::LastControlEvent(
                                                        ControlEvent::CleanRun { primary_ub },
                                                    ),
                                                )
                                                .await;
                                            } else {
                                                info!("Rebuilding the run from trusted data.");
                                                ctrl_tx
                                                    .send(ControlEvent::RebuildRun { primary_ub })
                                                    .map_err(ui_error)?;
                                                update_ui_state(
                                                    &state,
                                                    UiStateUpdate::LastControlEvent(
                                                        ControlEvent::RebuildRun { primary_ub },
                                                    ),
                                                )
                                                .await;
                                            }
                                        }
                                        scheduler::RecoveryState::GracefullyStopped => {
                                            if fresh {
                                                info!("Starting a fresh run, ignoring previous data.");
                                                ctrl_tx
                                                    .send(ControlEvent::CleanRun { primary_ub })
                                                    .map_err(ui_error)?;
                                                update_ui_state(
                                                    &state,
                                                    UiStateUpdate::LastControlEvent(
                                                        ControlEvent::CleanRun { primary_ub },
                                                    ),
                                                )
                                                .await;
                                            } else if rebuild {
                                                info!("Rebuilding the run from trusted data.");
                                                ctrl_tx
                                                    .send(ControlEvent::RebuildRun { primary_ub })
                                                    .map_err(ui_error)?;
                                                update_ui_state(
                                                    &state,
                                                    UiStateUpdate::LastControlEvent(
                                                        ControlEvent::RebuildRun { primary_ub },
                                                    ),
                                                )
                                                .await;
                                            } else {
                                                info!("Continuing the last run.");
                                                ctrl_tx
                                                    .send(ControlEvent::RestoreRun { primary_ub })
                                                    .map_err(ui_error)?;
                                                update_ui_state(
                                                    &state,
                                                    UiStateUpdate::LastControlEvent(
                                                        ControlEvent::RestoreRun { primary_ub },
                                                    ),
                                                )
                                                .await;
                                            }
                                        }
                                        _ => unreachable!(),
                                    },
                                    ControlEvent::Check { .. } => match rec_state {
                                        scheduler::RecoveryState::MissingData => {
                                            if rebuild {
                                                error!("Cannot rebuild.")
                                            } else {
                                                ctrl_tx
                                                    .send(ControlEvent::CleanRun { primary_ub })
                                                    .map_err(ui_error)?;
                                                update_ui_state(
                                                    &state,
                                                    UiStateUpdate::LastControlEvent(
                                                        ControlEvent::CleanRun { primary_ub },
                                                    ),
                                                )
                                                .await;
                                            }
                                        }
                                        scheduler::RecoveryState::AbortedChecked => {
                                            if fresh {
                                                info!("Starting a fresh run, ignoring previous data.");
                                                ctrl_tx
                                                    .send(ControlEvent::CleanRun { primary_ub })
                                                    .map_err(ui_error)?;
                                                update_ui_state(
                                                    &state,
                                                    UiStateUpdate::LastControlEvent(
                                                        ControlEvent::CleanRun { primary_ub },
                                                    ),
                                                )
                                                .await;
                                            } else {
                                                info!("Rebuilding the run from trusted data.");
                                                ctrl_tx
                                                    .send(ControlEvent::RebuildRun { primary_ub })
                                                    .map_err(ui_error)?;
                                                update_ui_state(
                                                    &state,
                                                    UiStateUpdate::LastControlEvent(
                                                        ControlEvent::RebuildRun { primary_ub },
                                                    ),
                                                )
                                                .await;
                                            }
                                        }
                                        scheduler::RecoveryState::GracefullyStoppedChecked => {
                                            if fresh {
                                                info!("Starting a fresh run, ignoring previous data.");
                                                ctrl_tx
                                                    .send(ControlEvent::CleanRun { primary_ub })
                                                    .map_err(ui_error)?;
                                                update_ui_state(
                                                    &state,
                                                    UiStateUpdate::LastControlEvent(
                                                        ControlEvent::CleanRun { primary_ub },
                                                    ),
                                                )
                                                .await;
                                            } else if rebuild {
                                                info!("Rebuilding the run from trusted data.");
                                                ctrl_tx
                                                    .send(ControlEvent::RebuildRun { primary_ub })
                                                    .map_err(ui_error)?;
                                                update_ui_state(
                                                    &state,
                                                    UiStateUpdate::LastControlEvent(
                                                        ControlEvent::RebuildRun { primary_ub },
                                                    ),
                                                )
                                                .await;
                                            } else {
                                                info!("Continuing the last run.");
                                                ctrl_tx
                                                    .send(ControlEvent::RestoreRun { primary_ub })
                                                    .map_err(ui_error)?;
                                                update_ui_state(
                                                    &state,
                                                    UiStateUpdate::LastControlEvent(
                                                        ControlEvent::RestoreRun { primary_ub },
                                                    ),
                                                )
                                                .await;
                                            }
                                        }
                                        _ => {
                                            warn!("Please wait until the check is finished.");
                                        }
                                    },
                                    _ => {
                                        warn!(
                                            "Already run. Use `exit` or `quit` to terminate the current session before starting a new run."
                                        );
                                    }
                                },
                                Action::Check => match exec_snapshot.last_control_event {
                                    ControlEvent::Check { .. } => {
                                        warn!("Already run a check.");
                                    }
                                    ControlEvent::Start => match rec_state {
                                        scheduler::RecoveryState::AbortedUnchecked
                                        | scheduler::RecoveryState::GracefullyStopped => {
                                            info!("Starting a consistency check of the remaining data.");
                                            ctrl_tx
                                                .send(ControlEvent::Check { primary_ub })
                                                .map_err(ui_error)?;
                                            update_ui_state(
                                                &state,
                                                UiStateUpdate::LastControlEvent(ControlEvent::Check {
                                                    primary_ub,
                                                }),
                                            )
                                            .await;
                                        }
                                        _ => {
                                            warn!(
                                                "Checks are only available when the previous run was aborted or gracefully stopped."
                                            );
                                        }
                                    },
                                    _ => {
                                        warn!("Cannot check after the run has already started.");
                                    }
                                },
                                Action::Exit => match exec_snapshot.last_control_event {
                                    ControlEvent::Start | ControlEvent::Check { .. } => {
                                        // We didn't start any jobs, so we can exit immediately.
                                        ctrl_tx.send(ControlEvent::Abort).map_err(ui_error)?;
                                        break;
                                    }
                                    ControlEvent::Abort | ControlEvent::GracefulStop
                                        if exec_snapshot.any_alive() =>
                                    {
                                        warn!(
                                            "Please wait until the current jobs are stopped before exiting."
                                        );
                                    }
                                    _ => match overall_state_snapshot {
                                        RunningState::Finished
                                        | RunningState::Stopped
                                        | RunningState::Error => {
                                            // `quit` first, then exit.
                                            if exec_snapshot.any_alive() {
                                                ctrl_tx.send(ControlEvent::Abort).map_err(ui_error)?;
                                                update_ui_state(
                                                    &state,
                                                    UiStateUpdate::LastControlEvent(ControlEvent::Abort),
                                                )
                                                .await;
                                                update_ui_state(&state, UiStateUpdate::ExitOnFinish(true))
                                                    .await;
                                            } else {
                                                break;
                                            }
                                        }
                                        RunningState::Running | RunningState::Paused => {
                                            warn!(
                                                "Cannot exit while jobs are running or paused. \
                                                Use `quit` for a graceful stop, or `quit --force` to abort all jobs."
                                            );
                                        }
                                    },
                                },
                                Action::Clear => {
                                    let mut guard = state.write().await;
                                    guard.log_buffer.clear();
                                    guard.cursor = 0;
                                    guard.unread_logs = 0;
                                }
                                Action::Quit { force, no_exit } => match exec_snapshot.last_control_event {
                                    ControlEvent::Start | ControlEvent::Check { .. } => {
                                        if !no_exit {
                                            ctrl_tx.send(ControlEvent::Abort).map_err(ui_error)?;
                                            break;
                                        } else {
                                            warn!("Nothing to quit.");
                                        }
                                    }
                                    ControlEvent::Abort if exec_snapshot.any_alive() => {
                                        warn!("Already processing an abort.");
                                    }
                                    ControlEvent::GracefulStop if exec_snapshot.any_alive() => {
                                        if force {
                                            warn!(
                                                "Already processing a graceful stop, but force quit requested."
                                            );
                                            ctrl_tx.send(ControlEvent::Abort).map_err(ui_error)?;
                                            update_ui_state(
                                                &state,
                                                UiStateUpdate::LastControlEvent(ControlEvent::Abort),
                                            )
                                            .await;
                                        } else {
                                            warn!("Already processing a graceful stop.");
                                        }
                                    }
                                    _ => match overall_state_snapshot {
                                        RunningState::Finished | RunningState::Stopped => {
                                            if exec_snapshot.any_alive() {
                                                ctrl_tx.send(ControlEvent::Abort).map_err(ui_error)?;
                                                update_ui_state(
                                                    &state,
                                                    UiStateUpdate::LastControlEvent(ControlEvent::Abort),
                                                )
                                                .await;
                                                update_ui_state(
                                                    &state,
                                                    UiStateUpdate::ExitOnFinish(!no_exit),
                                                )
                                                .await;
                                            } else if !no_exit {
                                                break;
                                            } else {
                                                warn!("Nothing to quit.");
                                            }
                                        }
                                        RunningState::Error => {
                                            if exec_snapshot.any_alive() {
                                                if !force {
                                                    warn!(
                                                        "Cannot gracefully stop due to previous errors. \
                                                        Defaulting to a force quit."
                                                    );
                                                }
                                                ctrl_tx.send(ControlEvent::Abort).map_err(ui_error)?;
                                                update_ui_state(
                                                    &state,
                                                    UiStateUpdate::LastControlEvent(ControlEvent::Abort),
                                                )
                                                .await;
                                                // We *don't* exit here (even without the no-exit flag),
                                                // because the user should be able to inspect the logs.
                                                if exec_snapshot.exit_on_finish {
                                                    update_ui_state(
                                                        &state,
                                                        UiStateUpdate::ExitOnFinish(false),
                                                    )
                                                    .await;
                                                }
                                            } else if !no_exit {
                                                break;
                                            } else {
                                                warn!("Nothing to quit.");
                                            }
                                        }
                                        RunningState::Paused | RunningState::Running => {
                                            if force {
                                                // Send an `Abort` event to all schedulers
                                                info!("Sent abort request, stopping immediately...");
                                                ctrl_tx.send(ControlEvent::Abort).map_err(ui_error)?;
                                                update_ui_state(
                                                    &state,
                                                    UiStateUpdate::LastControlEvent(ControlEvent::Abort),
                                                )
                                                .await;
                                            } else {
                                                // Send a `GracefulStop` command to all schedulers
                                                info!("Sent stop request, stopping gracefully...");
                                                ctrl_tx
                                                    .send(ControlEvent::GracefulStop)
                                                    .map_err(ui_error)?;
                                                update_ui_state(
                                                    &state,
                                                    UiStateUpdate::LastControlEvent(
                                                        ControlEvent::GracefulStop,
                                                    ),
                                                )
                                                .await;
                                            }
                                            update_ui_state(&state, UiStateUpdate::ExitOnFinish(!no_exit))
                                                .await;
                                        }
                                    },
                                },
                                Action::Pause { targets, cascade } => {
                                    match exec_snapshot.last_control_event {
                                        ControlEvent::Start | ControlEvent::Check { .. } => {
                                            warn!("Cannot pause before the run has started.");
                                        }
                                        ControlEvent::Abort | ControlEvent::GracefulStop
                                            if exec_snapshot.any_alive() =>
                                        {
                                            warn!("Cannot pause while stopping.");
                                        }
                                        _ => {
                                            if exec_snapshot
                                                .state_iter()
                                                .any(|s| s == RunningState::Running)
                                            {
                                                ctrl_tx
                                                    .send(ControlEvent::Pause {
                                                        targets: targets.clone(),
                                                        cascade,
                                                    })
                                                    .map_err(ui_error)?;
                                                update_ui_state(
                                                    &state,
                                                    UiStateUpdate::LastControlEvent(ControlEvent::Pause {
                                                        targets,
                                                        cascade,
                                                    }),
                                                )
                                                .await;
                                            } else {
                                                warn!(
                                                    "No running jobs to pause, did you mean to `exit` or `quit` instead?"
                                                );
                                            }
                                        }
                                    }
                                }
                                Action::Resume { targets } => match exec_snapshot.last_control_event {
                                    ControlEvent::Start | ControlEvent::Check { .. } => {
                                        warn!("Cannot resume before the run has started.");
                                    }
                                    ControlEvent::Abort | ControlEvent::GracefulStop
                                        if exec_snapshot.any_alive() =>
                                    {
                                        warn!("Cannot resume while stopping.");
                                    }
                                    _ => {
                                        if exec_snapshot
                                            .state_iter()
                                            .any(|s| s == RunningState::Paused)
                                        {
                                            ctrl_tx
                                                .send(ControlEvent::Resume {
                                                    targets: targets.clone(),
                                                })
                                                .map_err(ui_error)?;
                                            update_ui_state(
                                                &state,
                                                UiStateUpdate::LastControlEvent(ControlEvent::Resume {
                                                    targets,
                                                }),
                                            )
                                            .await;
                                        } else {
                                            warn!("No paused jobs to resume.");
                                        }
                                    }
                                },
                                Action::Help => {
                                    info!("{HELP_TEXT}");
                                }
                                _ => {
                                    warn!("Command not yet implemented: {command:?}");
                                }
                            }
                        }
                    }
                    _ = ::tokio::time::sleep(::std::time::Duration::from_millis(10)) => {
                        // Drain the log channel before drawing the UI.
                        loop {
                            match log_rx.try_recv() {
                                Ok(record) => {
                                    update_ui_state(
                                        &state,
                                        UiStateUpdate::NewLog(
                                            record,
                                            terminal.size().map_err(ui_error)?.width,
                                        ),
                                    )
                                    .await;
                                }
                                Err(::tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => {
                                    // Skip fallen-behind logs
                                    continue;
                                }
                                Err(::tokio::sync::broadcast::error::TryRecvError::Empty) => {
                                    // Drained all logs
                                    break;
                                }
                                Err(e) => {
                                    // Channel unexpectedly closed
                                    return Err(ui_error(e));
                                }
                            }
                        }

                        // Draw the UI state after the command execution
                        draw_ui(&mut terminal, &state).await?;
                    }
                }
            }

            // Cleanup:
            disable_raw_mode().map_err(ui_error)?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen).map_err(ui_error)?;
            terminal.show_cursor().map_err(ui_error)?;
            Ok(())
        }

        async fn draw_ui(
            terminal: &mut Terminal<impl Backend>,
            state: &Arc<RwLock<UiState>>,
        ) -> Result<()> {
            let draw_snapshot = {
                let guard = state.read().await;
                guard.clone()
            };
            // Draw the major pane(s)
            let mut cursor = draw_snapshot.cursor;
            terminal
                .draw(|frame| {
                    let [
                        progress_head,
                        progress_area,
                        progress_foot,
                        logs_head,
                        logs_area,
                        logs_foot,
                        input_area,
                    ] = Layout::vertical([
                        Constraint::Length(1),
                        Constraint::Length(6),
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Fill(1),
                        Constraint::Length(1),
                        Constraint::Length(1),
                    ])
                    .areas(frame.area());
                    frame.render_widget(
                        Block::new().borders(Borders::TOP).title(
                            Line::from(vec![
                                Span::raw("│ "),
                                Span::raw("Progress").italic(),
                                Span::raw(" ├"),
                            ])
                            .left_aligned(),
                        ),
                        progress_head,
                    );
                    let [
                        progress_description,
                        beta_bar,
                        gamma_bar,
                        delta_bar,
                        epsilon_bar,
                        zeta_bar,
                    ] = Layout::vertical([Constraint::Length(1); 6]).areas(progress_area);
                    frame.render_widget(
                        Line::from(if progress_description.width >= 76 + 7 {
                            vec![
                                Span::raw(format!("{:4}", "")),
                                Span::raw("job").underlined(),
                                Span::raw("   "),
                                Span::raw("done").underlined(),
                                Span::raw(" "),
                                Span::raw("ready").underlined(),
                                Span::raw("  "),
                                Span::raw("wait").underlined(),
                                Span::raw("   Colors: ").dark_gray().italic(),
                                Span::raw("Finished").green().italic(),
                                Span::raw(" | ").dark_gray(),
                                Span::raw("Running").cyan().italic(),
                                Span::raw(" | ").dark_gray(),
                                Span::raw("Paused").yellow().italic(),
                                Span::raw(" | ").dark_gray(),
                                Span::raw("Error").red().italic(),
                                Span::raw(" | ").dark_gray(),
                                Span::raw("Stopped").dark_gray().italic(),
                            ]
                        } else {
                            vec![
                                Span::raw(format!("{:4}", "")),
                                Span::raw("job").underlined(),
                                Span::raw("   "),
                                Span::raw("done").underlined(),
                                Span::raw(" "),
                                Span::raw("ready").underlined(),
                                Span::raw("  "),
                                Span::raw("wait").underlined(),
                            ]
                        })
                        .left_aligned(),
                        progress_description,
                    );
                    draw_progress_gauge(frame, beta_bar, "Beta", draw_snapshot.beta);
                    draw_progress_gauge(frame, gamma_bar, "Gamma", draw_snapshot.gamma);
                    draw_progress_gauge(frame, delta_bar, "Delta", draw_snapshot.delta);
                    draw_progress_gauge(frame, epsilon_bar, "Epsilon", draw_snapshot.epsilon);
                    draw_progress_gauge(frame, zeta_bar, "Zeta", draw_snapshot.zeta);
                    frame.render_widget(Block::new().borders(Borders::BOTTOM), progress_foot);
                    frame.render_widget(
                        Block::new().borders(Borders::TOP).title(
                            Line::from(vec![
                                Span::raw("│ "),
                                Span::raw("Logs").italic(),
                                Span::raw(" ├"),
                            ])
                            .left_aligned(),
                        ),
                        logs_head,
                    );
                    let (logs_widget, new_cursor) = draw_snapshot.log_buffer.to_text(
                        logs_area.width,
                        logs_area.height,
                        draw_snapshot.cursor,
                    );
                    cursor = new_cursor;
                    frame.render_widget(logs_widget, logs_area);
                    frame.render_widget(
                        Block::new().borders(Borders::BOTTOM).title(
                            Line::from(if draw_snapshot.unread_logs == 0 {
                                vec![]
                            } else {
                                vec![
                                    Span::raw("┤ "),
                                    Span::raw(format!(
                                        "{} unread, Esc to follow",
                                        draw_snapshot.unread_logs
                                    ))
                                    .italic(),
                                    Span::raw(" │"),
                                ]
                            })
                            .right_aligned(),
                        ),
                        logs_foot,
                    );
                    draw_snapshot.shell.render(
                        frame,
                        input_area,
                        overall_state(&draw_snapshot).color(),
                    );
                })
                .map_err(ui_error)?;
            // Update the cursor position if it was clipped
            if cursor != draw_snapshot.cursor {
                update_ui_state(state, UiStateUpdate::SetCursor(cursor)).await;
            }
            Ok(())
        }

        /// Formats the count for display in the progress gauge.
        fn five_format(count: i64) -> String {
            debug_assert!(count >= 0, "Count must be non-negative");
            fn intdiv_truncate_down(raw: i64, divis_power: u32, digits_under_point: u32) -> f64 {
                let divisor = 10i64.pow(divis_power.saturating_sub(digits_under_point));
                let divided = raw / divisor;
                let factor = 10f64.powi(digits_under_point as i32);
                divided as f64 / factor
            }
            let raw_len = count.to_string().len();
            match raw_len {
                i if i < 5 => format!("{count:>5}"),
                5 => format!("{:.1}K", intdiv_truncate_down(count, 3, 1)),
                6 => format!(" {}K", intdiv_truncate_down(count, 3, 0) as i64),
                7 => format!("{:.2}M", intdiv_truncate_down(count, 6, 2)),
                8 => format!("{:.1}M", intdiv_truncate_down(count, 6, 1)),
                9 => format!(" {}M", intdiv_truncate_down(count, 6, 0) as i64),
                10 => format!("{:.2}B", intdiv_truncate_down(count, 9, 2)),
                11 => format!("{:.1}B", intdiv_truncate_down(count, 9, 1)),
                12 => format!(" {}B", intdiv_truncate_down(count, 9, 0) as i64),
                13 => format!("{:.2}T", intdiv_truncate_down(count, 12, 2)),
                14 => format!("{:.1}T", intdiv_truncate_down(count, 12, 1)),
                15 => format!(" {}T", intdiv_truncate_down(count, 12, 0) as i64),
                16 => format!("{:.1}Qa", intdiv_truncate_down(count, 15, 1)),
                17 | 18 => format!("{:>3}Qa", intdiv_truncate_down(count, 15, 0) as i64),
                19 => format!("{:.1}Qi", intdiv_truncate_down(count, 18, 1)),
                _ => unreachable!(),
            }
        }

        /// Draws a progress gauge with a text label and a manual gauge.
        fn draw_progress_gauge(frame: &mut Frame<'_>, area: Rect, name: &str, progress: Progress) {
            // Text area: "epsilon [ done/queue/ wait] "
            let text_length = 21 + 7; // 7 stands for the longest job name length
            let horizontal = Layout::horizontal([
                Constraint::Length(text_length),
                Constraint::Length(1),
                Constraint::Fill(1),
                Constraint::Length(1),
            ]);
            let [text_area, gauge_left, gauge_area, gauge_right] = horizontal.areas(area);
            let (done, queued, waiting) = (progress.0, progress.1, progress.2);
            frame.render_widget(
                Line::from(vec![
                    Span::styled(format!("{name:>7}"), Style::new().fg(progress.3.color())),
                    Span::raw(format!(
                        " [{}/{}/{}] ",
                        five_format(done),
                        five_format(queued),
                        five_format(waiting)
                    )),
                ]),
                text_area,
            );

            // Gauge area: manual gauge with Span, surrounded by borders
            let gauge_length = gauge_area.width;
            let done_length = if done + queued + waiting > 0 {
                ((done as f64 / (done + queued + waiting) as f64) * gauge_length as f64).round()
                    as u16
            } else {
                0
            };
            let queued_length = if done + queued + waiting > 0 {
                ((queued as f64 / (done + queued + waiting) as f64) * gauge_length as f64).round()
                    as u16
            } else {
                0
            };
            let waiting_length = gauge_length.saturating_sub(done_length + queued_length);
            let [done_area, queued_area, waiting_area] = Layout::horizontal([
                Constraint::Length(done_length),
                Constraint::Length(queued_length),
                Constraint::Fill(1),
            ])
            .areas(gauge_area);
            let done_span = Span::styled(
                "█".repeat(done_length as usize),
                Style::default().fg(progress.3.color()),
            );
            let queued_span = Span::styled(
                "░".repeat(queued_length as usize),
                Style::default().fg(progress.3.color()),
            );
            let waiting_span = Span::styled(" ".repeat(waiting_length as usize), Style::default());
            frame.render_widget(
                Block::new()
                    .borders(Borders::LEFT)
                    .border_style(Style::default().fg(Color::White)),
                gauge_left,
            );
            frame.render_widget(Paragraph::new(done_span), done_area);
            frame.render_widget(Paragraph::new(queued_span), queued_area);
            frame.render_widget(Paragraph::new(waiting_span), waiting_area);
            frame.render_widget(
                Block::new()
                    .borders(Borders::RIGHT)
                    .border_style(Style::default().fg(Color::White)),
                gauge_right,
            );
        }
    }
}
