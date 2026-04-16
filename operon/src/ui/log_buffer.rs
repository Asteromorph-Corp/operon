use std::collections::VecDeque;

use ratatui::text::{Line, Text};

use crate::logger::LogRecord;

#[derive(Debug, Clone)]
pub struct LogBuffer {
    records: VecDeque<LogRecord>,
    capacity: usize,
}

impl LogBuffer {
    pub fn new(size: usize) -> Self {
        Self {
            records: VecDeque::with_capacity(size),
            capacity: size,
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

    pub fn to_lines(&self, width: u16, verbose: bool) -> Vec<Line<'static>> {
        self.records
            .iter()
            .flat_map(|r| r.format_for_term(width, verbose))
            .collect()
    }

    /// Display the bottom `height` lines, skipping `cursor` lines.
    /// If `cursor + height` exceeds the number of lines, `cursor` will be clamped down.
    pub fn to_text(
        &self,
        width: u16,
        height: u16,
        cursor: usize,
        verbose: bool,
    ) -> (Text<'static>, usize) {
        let lines = self.to_lines(width, verbose);
        let cursor = cursor.min(lines.len().saturating_sub(height as usize));
        let num_buffer_lines = height.saturating_sub((lines.len() - cursor) as u16);
        let visible_lines = lines
            .into_iter()
            .rev()
            .skip(cursor)
            .take(height as usize)
            .chain(::std::iter::repeat_n(
                Line::from(""),
                num_buffer_lines as usize,
            ))
            .rev()
            .collect::<Vec<_>>();
        (Text::from(visible_lines), cursor)
    }
}
