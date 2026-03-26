use tracing::field::{Field, Visit};

/// Metadata extracted from `log.*` fields on `tracing_log`-bridged events.
///
/// When crates use the `log` facade and `tracing_log` bridges them into tracing,
/// the tracing metadata target is `"log"` and the original crate metadata is
/// stored as `log.target`, `log.module_path`, `log.file`, and `log.line` fields.
#[derive(Debug, Default)]
pub(super) struct LogBridgedMeta {
    pub target: Option<String>,
    pub module_path: Option<String>,
    pub file: Option<String>,
    pub line: Option<u64>,
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
pub(super) struct EventVisitor {
    pub message: String,
    pub fields: Vec<(String, String)>,
    pub log_meta: LogBridgedMeta,
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
pub(super) struct FieldVisitor {
    pub fields: Vec<(String, String)>,
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
