use ratatui::style::Stylize;
use ratatui::text::{Line, Span};

pub type LogRecordReceiver = tokio::sync::broadcast::Receiver<LogRecord>;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct LogRecord {
    timestamp: ::chrono::DateTime<::chrono::offset::Local>,
    level: ::tracing::Level,
    target: String,
    file: Option<String>,
    module_path: Option<String>,
    line: Option<u32>,
    span_context: Option<String>,
    msg: String,
}

impl LogRecord {
    pub fn new(
        level: ::tracing::Level,
        target: String,
        file: Option<String>,
        module_path: Option<String>,
        line: Option<u32>,
        span_context: Option<String>,
        msg: String,
    ) -> Self {
        LogRecord {
            timestamp: ::chrono::Local::now(),
            level,
            target,
            file,
            module_path,
            line,
            span_context,
            msg,
        }
    }

    pub fn format_for_term(&self, width: u16) -> Vec<Line<'static>> {
        let timestamp = self.timestamp.format("%y-%m-%d %H:%M:%S").to_string();
        let level_colour = match self.level {
            ::tracing::Level::ERROR => ::ratatui::style::Style::new().red(),
            ::tracing::Level::WARN => ::ratatui::style::Style::new().yellow(),
            ::tracing::Level::INFO => ::ratatui::style::Style::new().green(),
            ::tracing::Level::DEBUG => ::ratatui::style::Style::new().cyan(),
            ::tracing::Level::TRACE => ::ratatui::style::Style::new().white(),
        };
        // Colour messages that echo shell input.
        let msg_colour = if self.msg.starts_with("$ ") {
            match self.level {
                ::tracing::Level::ERROR => ::ratatui::style::Style::new().light_red(),
                ::tracing::Level::INFO => ::ratatui::style::Style::new().light_green(),
                _ => ::ratatui::style::Style::new(),
            }
        } else {
            ::ratatui::style::Style::new()
        };
        let level_label = self.level.as_str();
        let span_style = ::ratatui::style::Style::new().dark_gray();

        let span_ctx = match &self.span_context {
            Some(ctx) => format!("[{ctx}] "),
            None => String::new(),
        };

        let prefix = format!("{timestamp} ");
        let level = format!("{level_label:>5}");
        let sep = "│ ";
        let prefix_width = prefix.chars().count() + level.chars().count() + sep.chars().count();
        let available = (width as usize).saturating_sub(prefix_width);
        let wrap_indent = " ".repeat(prefix_width.saturating_sub(4)) + "...│ ";
        let wrap_options = ::textwrap::Options::new(available);

        let mut lines: Vec<Line<'_>> = vec![];
        for (i, line) in self.msg.lines().enumerate() {
            let content = if i == 0 {
                format!("{span_ctx}{line}")
            } else {
                line.to_string()
            };

            for (l, wrapped) in ::textwrap::wrap(&content, &wrap_options).iter().enumerate() {
                let mut spans = if l == 0 {
                    vec![
                        Span::raw(prefix.clone()),
                        Span::styled(level.clone(), level_colour),
                        Span::raw(sep.to_string()),
                    ]
                } else {
                    vec![Span::raw(wrap_indent.clone())]
                };

                let segments = if l == 0 && i == 0 {
                    style_message_segments(wrapped, &span_ctx, span_style, msg_colour)
                } else {
                    vec![(wrapped.to_string(), msg_colour)]
                };
                for (text, style) in segments {
                    if !text.is_empty() {
                        spans.push(Span::styled(text, style));
                    }
                }

                lines.push(Line::from(spans));
            }
        }
        lines
    }

    pub fn format_for_dump(&self) -> String {
        let msg = match &self.span_context {
            Some(ctx) => format!("[{ctx}] {}", self.msg),
            None => self.msg.clone(),
        };
        format!(
            "{},{},{},{},{},{},\"{}\"",
            self.timestamp.format("%Y-%m-%d %H:%M:%S%.f %:z"),
            self.level,
            self.target,
            self.file.as_deref().unwrap_or(""),
            self.module_path.as_deref().unwrap_or(""),
            self.line.unwrap_or(0),
            msg.replace("\"", "\"\"")
        )
    }

    pub fn format_for_print(&self) -> String {
        let level_colour = match self.level {
            ::tracing::Level::ERROR => ansi_term::Colour::Red,
            ::tracing::Level::WARN => ansi_term::Colour::Yellow,
            ::tracing::Level::INFO => ansi_term::Colour::Green,
            ::tracing::Level::DEBUG => ansi_term::Colour::Cyan,
            ::tracing::Level::TRACE => ansi_term::Colour::White,
        };
        let msg = match &self.span_context {
            Some(ctx) => format!("[{ctx}] {}", self.msg),
            None => self.msg.clone(),
        };
        format!(
            "{} {} | {}",
            self.timestamp.format("%Y-%m-%d %H:%M:%S"),
            level_colour.paint(format!("{:>5}", self.level)),
            msg
        )
    }
}

/// Split the first line of a message into styled segments:
/// `[span context] ` (gray) + `!stdout ` / `!stderr ` (yellow/red) + message (default).
fn style_message_segments(
    text: &str,
    span_ctx: &str,
    span_style: ::ratatui::style::Style,
    msg_style: ::ratatui::style::Style,
) -> Vec<(String, ::ratatui::style::Style)> {
    let mut result = Vec::new();
    let mut remaining = text.to_string();

    // 1. Strip span context prefix
    if !span_ctx.is_empty() {
        let n = span_ctx.chars().count().min(remaining.chars().count());
        result.push((remaining.chars().take(n).collect(), span_style));
        remaining = remaining.chars().skip(n).collect();
    }

    // 2. Strip !stdout / !stderr capture prefix
    let capture_prefixes: &[(&str, ::ratatui::style::Style)] = &[
        ("!stdout ", ::ratatui::style::Style::new().yellow()),
        ("!stderr ", ::ratatui::style::Style::new().red()),
    ];
    for &(prefix, style) in capture_prefixes {
        if remaining.starts_with(prefix) {
            result.push((prefix.to_string(), style));
            remaining = remaining[prefix.len()..].to_string();
            break;
        }
    }

    // 3. The rest is the actual message
    result.push((remaining, msg_style));
    result
}
