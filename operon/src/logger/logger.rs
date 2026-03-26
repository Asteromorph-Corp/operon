use std::io::Write;
use std::path::PathBuf;

use tracing::Subscriber;
use tracing::field::{Field, Visit};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;

use crate::logger::LoggerOptions;
use crate::ui::{LogRecord, UiError};

/// Stored on each span to hold its recorded fields.
#[derive(Debug, Default, Clone)]
struct SpanFields(Vec<(String, String)>);

/// Metadata extracted from `log.*` fields on `tracing_log`-bridged events.
///
/// When crates use the `log` facade and `tracing_log` bridges them into tracing,
/// the tracing metadata target is `"log"` and the original crate metadata is
/// stored as `log.target`, `log.module_path`, `log.file`, and `log.line` fields.
#[derive(Debug, Default)]
struct LogBridgedMeta {
    target: Option<String>,
    module_path: Option<String>,
    file: Option<String>,
    line: Option<u64>,
}

impl LogBridgedMeta {
    /// Resolve each field, preferring the bridged value over tracing metadata.
    pub fn resolve(
        self,
        metadata: &tracing::Metadata<'_>,
    ) -> (String, Option<String>, Option<String>, Option<u32>) {
        let target = self.target.unwrap_or_else(|| metadata.target().to_string());
        let file = self.file.or_else(|| metadata.file().map(String::from));
        let module_path = self
            .module_path
            .or_else(|| metadata.module_path().map(String::from));
        let line = self.line.map(|n| n as u32).or(metadata.line());
        (target, file, module_path, line)
    }
}

/// Visitor that collects the message, structured fields, and any `log.*` bridged metadata.
#[derive(Debug, Default)]
struct EventVisitor {
    message: String,
    fields: Vec<(String, String)>,
    log_meta: LogBridgedMeta,
}

impl Visit for EventVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        match field.name() {
            "message" => self.message = format!("{value:?}"),
            "log.target" => self.log_meta.target = Some(format!("{value:?}")),
            "log.module_path" => self.log_meta.module_path = Some(format!("{value:?}")),
            "log.file" => self.log_meta.file = Some(format!("{value:?}")),
            "log.line" => {
                if let Ok(n) = format!("{value:?}").parse() {
                    self.log_meta.line = Some(n);
                }
            }
            _ => self
                .fields
                .push((field.name().to_string(), format!("{value:?}"))),
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "message" => self.message = value.to_string(),
            "log.target" => self.log_meta.target = Some(value.to_string()),
            "log.module_path" => self.log_meta.module_path = Some(value.to_string()),
            "log.file" => self.log_meta.file = Some(value.to_string()),
            "log.line" => {
                if let Ok(n) = value.parse() {
                    self.log_meta.line = Some(n);
                }
            }
            _ => self
                .fields
                .push((field.name().to_string(), value.to_string())),
        }
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        if field.name() == "log.line" {
            self.log_meta.line = Some(value);
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

    fn on_record(
        &self,
        id: &tracing::span::Id,
        values: &tracing::span::Record<'_>,
        ctx: Context<'_, S>,
    ) {
        if let Some(span) = ctx.span(id) {
            let mut visitor = FieldVisitor::default();
            values.record(&mut visitor);

            let mut extensions = span.extensions_mut();
            if let Some(existing) = extensions.get_mut::<SpanFields>() {
                for (new_key, new_val) in visitor.fields {
                    if let Some((_, existing_val)) =
                        existing.0.iter_mut().find(|(k, _)| *k == new_key)
                    {
                        *existing_val = new_val;
                    } else {
                        existing.0.push((new_key, new_val));
                    }
                }
            } else {
                extensions.insert(SpanFields(visitor.fields));
            }
        }
    }

    fn on_event(&self, event: &tracing::Event<'_>, ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let level = *metadata.level();

        // Level filter
        if level > self.level {
            return;
        }

        // Extract event message, fields, and any `log.*` bridged metadata
        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);

        // Resolve metadata: prefer bridged `log.*` fields over tracing metadata
        // so that events from crates using the `log` facade get their original
        // target/module/file/line instead of the generic "log" placeholder.
        let (target, file, module_path, line) = visitor.log_meta.resolve(metadata);

        // Blacklist noisy third-party crate logs
        if target.starts_with("tokio_postgres") && level >= tracing::Level::INFO {
            return;
        }
        if target.starts_with("mio::poll") && level >= tracing::Level::TRACE {
            return;
        }

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

        // Append structured event fields to the message
        let message = if visitor.fields.is_empty() {
            visitor.message
        } else {
            let fields = visitor
                .fields
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(", ");
            if visitor.message.is_empty() {
                fields
            } else {
                format!("{} {{{fields}}}", visitor.message)
            }
        };

        let record = LogRecord::new(
            level,
            target,
            file,
            module_path,
            line,
            span_context,
            message,
        );

        // Dump to CSV
        self.dump_record(&record);

        // Send to UI (fails silently when UI closes)
        let _ = self.sender.send(record);
    }
}
