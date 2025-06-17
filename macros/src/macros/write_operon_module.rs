use proc_macro2::TokenStream;
use proc_macro_crate::{crate_name, FoundCrate};
use quote::{format_ident, quote};

use crate::{Entities, GlobalConfig};

pub fn write_operon_module(_entities: &Entities, config: &GlobalConfig) -> TokenStream {
    let crate_path = match crate_name("operon-macros") {
        Ok(FoundCrate::Itself) => quote! { crate },
        Ok(FoundCrate::Name(name)) => {
            let path = format_ident!("{}", name);
            quote! { #path }},
        Err(_) => quote! { operon }
    };

    let meta_uri = &config.storage.metadata.uri;
    let meta_schema = match &config.storage.metadata.schema {
        Some(schema) => quote! { Some(#schema) },
        None => quote! { None },
    };
    let data_uri = &config.storage.data.uri;
    let data_schema = match &config.storage.data.schema {
        Some(schema) => quote! { Some(#schema) },
        None => quote! { None },
    };
    let log_buffer_size = config.log.buffer_size;
    let log_level = match &config.log.level {
        s if s == "trace" => quote! { #crate_path::log::Level::Trace },
        s if s == "debug" => quote! { #crate_path::log::Level::Debug },
        s if s == "info" => quote! { #crate_path::log::Level::Info },
        s if s == "warn" => quote! { #crate_path::log::Level::Warn },
        s if s == "error" => quote! { #crate_path::log::Level::Error },
        _ => panic!("Invalid log level: {}", config.log.level),
    };
    let log_dump = config.log.dump;
    let log_dump_dir = &config.log.dump_path.as_os_str().to_string_lossy();

    quote! {
        pub mod operon {
            use super::operon_internal::*;
            /// The size of the internal channel buffers.
            pub const INTERNAL_CHANNEL_SIZE: usize = 1024;
            /// The URI of the PostgreSQL database used by the metadata storage.
            pub const META_DATABASE_URI: &str = #meta_uri;
            /// The name of the metadata schema in the PostgreSQL database.
            pub const META_SCHEMA: Option<&'static str> = #meta_schema;
            /// The URI of the PostgreSQL database used by the data storage.
            pub const DATABASE_URI: &str = #data_uri;
            /// The name of the data schema in the PostgreSQL database.
            pub const DATA_SCHEMA: Option<&'static str> = #data_schema;
            /// The number of logs that the UI keeps in memory.
            pub const LOG_BUFFER_SIZE: usize = #log_buffer_size;
            /// The minimum log level to display.
            pub const LOG_LEVEL: #crate_path::log::Level = #log_level;
            /// Enable or disable log-dumping to a file.
            pub const LOG_DUMP: bool = #log_dump;
            pub const LOG_DUMP_DIR: &str = #log_dump_dir;

            /// Error type returned by Operon.
            #[derive(Debug, #crate_path::thiserror::Error)]
            pub enum OperonError {
                /// Error in a storage operation
                #[error("Storage error: {0}")]
                Storage(#[source] #crate_path::anyhow::Error),

                /// Error in the scheduler
                #[error("Scheduler error: {0}")]
                Scheduler(#[source] #crate_path::anyhow::Error),

                /// Error in a user function
                #[error("User function error: {0}")]
                User(#[source] #crate_path::anyhow::Error),

                /// Error in the metadata storage
                #[error("Metadata storage error: {0}")]
                MetaStorage(#[source] #crate_path::anyhow::Error),

                /// Error in the terminal UI
                #[error("Terminal UI error: {0}")]
                UI(#[source] #crate_path::anyhow::Error),

                /// Error caused by missing data
                #[error("Data expected but not found: {0}")]
                NotFound(String),

                /// Tried to resolve a ticket with an irrelevant resolution
                #[error("Invalid resolution: {0}")]
                InvalidResolution(String),
            }
            pub(super) fn scheduler_error<E: ::std::error::Error + Send + Sync + 'static>(
                e: E,
            ) -> OperonError {
                OperonError::Scheduler(#crate_path::anyhow::Error::from(e))
            }
            pub(super) fn scheduler_error_str<S: Into<String>>(s: S) -> OperonError {
                OperonError::Scheduler(#crate_path::anyhow::Error::msg(s.into()))
            }
            pub(super) fn meta_storage_error<E: ::std::error::Error + Send + Sync + 'static>(
                e: E,
            ) -> OperonError {
                OperonError::MetaStorage(#crate_path::anyhow::Error::from(e))
            }
            pub(super) fn meta_storage_error_str<S: Into<String>>(s: S) -> OperonError {
                OperonError::MetaStorage(#crate_path::anyhow::Error::msg(s.into()))
            }
            pub(super) fn ui_error<E: ::std::error::Error + Send + Sync + 'static>(e: E) -> OperonError {
                OperonError::UI(#crate_path::anyhow::Error::from(e))
            }
        }
    }
}
