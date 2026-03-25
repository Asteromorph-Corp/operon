use std::io::Write;
use std::path::PathBuf;

use crate::logger::LoggerOptions;
use crate::ui::{LogRecord, UiError};

#[derive(Debug)]
pub struct Logger {
    sender: ::tokio::sync::broadcast::Sender<LogRecord>,
    level: log::Level,
    dump: Option<String>,
}

impl Logger {
    pub fn new(
        sender: ::tokio::sync::broadcast::Sender<LogRecord>,
        options: LoggerOptions,
    ) -> Self {
        Self {
            sender,
            level: options.level,
            dump: options.dump,
        }
    }

    /// Sets up the logger with the given level filter.
    pub fn setup(self, level_filter: log::LevelFilter) -> Result<(), UiError> {
        // Set the logger
        ::log::set_boxed_logger(Box::new(self))?;
        // Set the maximum log level
        ::log::set_max_level(level_filter);
        Ok(())
    }

    pub fn blacklisted(&self, metadata: &::log::Metadata) -> bool {
        // Blacklist logs from external crates that come from inside Operon.
        // These are usually too verbose and not useful for the user.
        // These blacklists should be configurable by the user.
        if metadata.target().starts_with("tokio_postgres") && metadata.level() >= ::log::Level::Info
        {
            return true;
        }
        if metadata.target().starts_with("mio::poll") && metadata.level() >= ::log::Level::Trace {
            return true;
        }
        false
    }

    fn dump(&self, record: &log::Record) {
        let Some(dump) = &self.dump else {
            return;
        };

        if self.blacklisted(record.metadata()) {
            return;
        }

        let dump_dir = PathBuf::from(&dump);

        if !dump_dir.exists() {
            ::std::fs::create_dir_all(&dump_dir).unwrap();
        }
        let log_file = match ::std::fs::OpenOptions::new()
            .append(true)
            .create_new(true)
            .open(dump_dir.join("operon.csv"))
        {
            Ok(file) => {
                let mut writer = ::std::io::BufWriter::new(file);
                let _ = writeln!(writer, "timestamp,level,target,file,module,line,message");
                writer.into_inner().unwrap()
            }
            Err(ref e) if e.kind() == ::std::io::ErrorKind::AlreadyExists => {
                ::std::fs::OpenOptions::new()
                    .append(true)
                    .open(dump_dir.join("operon.csv"))
                    .unwrap()
            }
            Err(_) => {
                panic!("Couldn't open log file")
            }
        };
        let mut writer = ::std::io::BufWriter::new(log_file);
        let record = LogRecord::from(record);
        let _ = writeln!(writer, "{}", record.format_for_dump());
    }
}

impl ::log::Log for Logger {
    fn enabled(&self, metadata: &::log::Metadata) -> bool {
        if self.blacklisted(metadata) {
            return false;
        }
        metadata.level() <= self.level
    }

    fn log(&self, record: &::log::Record) {
        self.dump(record);

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
