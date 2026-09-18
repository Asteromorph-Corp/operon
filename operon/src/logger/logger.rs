use std::io::Write;
use std::path::PathBuf;

use tracing::Subscriber;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;

use crate::logger::visitors::{EventVisitor, FieldVisitor};
use crate::logger::{LogRecord, LogRecordSender, LoggerOptions, SourceType};
use crate::ui::UiError;

/// Stored on each span to hold its recorded fields.
#[derive(Debug, Default, Clone)]
struct SpanFields(Vec<(String, String)>);

#[derive(Debug)]
pub(crate) struct UiBroadcastLayer {
    sender: LogRecordSender,
    level: tracing::Level,
    dump: Option<String>,
}

impl UiBroadcastLayer {
    pub(crate) fn new(sender: LogRecordSender, options: LoggerOptions) -> Self {
        Self {
            sender,
            level: options.level,
            dump: options.dump,
        }
    }

    pub(crate) fn setup(self) -> Result<(), UiError> {
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
        let _ = record.write_dump(&mut writer);
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

        // Extract the source type
        let source_type = if target.starts_with("stdio::stdout") {
            SourceType::Stdout
        } else if target.starts_with("stdio::stderr") {
            SourceType::Stderr
        } else {
            SourceType::Levelled
        };

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
            source_type,
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
