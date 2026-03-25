use std::io::Write;
use std::path::PathBuf;

use tracing::field::{Field, Visit};
use tracing::Subscriber;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

use crate::logger::LoggerOptions;
use crate::ui::{LogRecord, UiError};

/// Stored on each span to hold its recorded fields.
#[derive(Debug, Default, Clone)]
struct SpanFields(Vec<(String, String)>);

/// Visitor that collects the message and structured fields from a tracing event.
#[derive(Debug, Default)]
struct EventVisitor {
    message: String,
    fields: Vec<(String, String)>,
}

impl Visit for EventVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        } else {
            self.fields
                .push((field.name().to_string(), format!("{value:?}")));
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else {
            self.fields
                .push((field.name().to_string(), value.to_string()));
        }
    }
}

/// Visitor that collects fields recorded on a span.
#[derive(Debug, Default)]
struct FieldVisitor {
    fields: Vec<(String, String)>,
}

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.fields
            .push((field.name().to_string(), format!("{value:?}")));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.fields
            .push((field.name().to_string(), value.to_string()));
    }
}

#[derive(Debug)]
pub struct UiBroadcastLayer {
    sender: ::tokio::sync::broadcast::Sender<LogRecord>,
    level: tracing::Level,
    dump: Option<String>,
}

impl UiBroadcastLayer {
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

    pub fn setup(self) -> Result<(), UiError> {
        use tracing_subscriber::prelude::*;

        // Bridge the `log` crate so third-party crates using `log` are captured.
        tracing_log::LogTracer::init().map_err(|e| UiError::Other(e.to_string()))?;

        let subscriber = tracing_subscriber::registry().with(self);
        tracing::subscriber::set_global_default(subscriber)?;
        Ok(())
    }

    fn dump_record(&self, record: &LogRecord) {
        let Some(dump) = &self.dump else {
            return;
        };

        let dump_dir = PathBuf::from(dump);

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
        let _ = writeln!(writer, "{}", record.format_for_dump());
    }
}

impl<S> Layer<S> for UiBroadcastLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: Context<'_, S>,
    ) {
        let mut visitor = FieldVisitor::default();
        attrs.record(&mut visitor);
        if let Some(span) = ctx.span(id) {
            span.extensions_mut().insert(SpanFields(visitor.fields));
        }
    }

    fn on_event(&self, event: &tracing::Event<'_>, ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let level = *metadata.level();

        // Level filter
        if level > self.level {
            return;
        }

        // Blacklist noisy third-party crate logs
        let target = metadata.target();
        if target.starts_with("tokio_postgres") && level >= tracing::Level::INFO {
            return;
        }
        if target.starts_with("mio::poll") && level >= tracing::Level::TRACE {
            return;
        }

        // Extract event message and fields
        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);

        // Collect span context
        let mut span_parts: Vec<String> = Vec::new();
        if let Some(scope) = ctx.event_scope(event) {
            for span in scope.from_root() {
                let extensions = span.extensions();
                if let Some(fields) = extensions.get::<SpanFields>() {
                    for (k, v) in &fields.0 {
                        span_parts.push(format!("{k}={v}"));
                    }
                }
            }
        }

        let span_context = if span_parts.is_empty() {
            None
        } else {
            Some(span_parts.join(", "))
        };

        let record = LogRecord::new(
            level,
            metadata.target().to_string(),
            metadata.file().map(|s| s.to_string()),
            metadata.module_path().map(|s| s.to_string()),
            metadata.line(),
            span_context,
            visitor.message,
        );

        // Dump to CSV
        self.dump_record(&record);

        // Send to UI (fails silently when UI closes)
        let _ = self.sender.send(record);
    }
}
