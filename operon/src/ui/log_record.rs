use ratatui::style::Stylize;
use ratatui::text::{Line, Span};

pub type LogRecordReceiver = tokio::sync::broadcast::Receiver<LogRecord>;

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
    pub fn format_for_term(&self, width: u16) -> Vec<Line<'static>> {
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

    pub fn format_for_dump(&self) -> String {
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

    pub fn format_for_print(&self) -> String {
        let level_colour = match self.level {
            log::Level::Error => ansi_term::Colour::Red,
            log::Level::Warn => ansi_term::Colour::Yellow,
            log::Level::Info => ansi_term::Colour::Green,
            log::Level::Debug => ansi_term::Colour::Cyan,
            log::Level::Trace => ansi_term::Colour::White,
        };
        format!(
            "{} {} | {}",
            self.timestamp.format("%Y-%m-%d %H:%M:%S"),
            level_colour.paint(format!("{:>5}", self.level)),
            self.msg
        )
    }
}
