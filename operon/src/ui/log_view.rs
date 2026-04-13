use ratatui::text::Text;

use crate::logger::LogRecord;
use crate::ui::log_buffer::LogBuffer;

/// Owns the log buffer and scroll state.
pub struct LogView {
    buffer: LogBuffer,
    /// Number of bottom lines to skip (scroll offset).
    cursor: usize,
    unread: usize,
}

impl LogView {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            buffer: LogBuffer::new(buffer_size),
            cursor: 0,
            unread: 0,
        }
    }

    /// Push a new log record. If the cursor is scrolled back,
    /// adjust it to keep the viewport stable and track unread count.
    pub fn push(&mut self, record: LogRecord, width: u16, verbose: bool) {
        if self.cursor != 0 {
            self.cursor = self
                .cursor
                .saturating_add(record.format_for_term(width, verbose).len());
            self.unread += 1;
        }
        self.buffer.push(record);
    }

    /// Scroll up (away from latest) by `n` lines.
    pub fn scroll_up(&mut self, n: usize) {
        self.cursor = self.cursor.saturating_add(n);
    }

    /// Scroll down (toward latest) by `n` lines.
    pub fn scroll_down(&mut self, n: usize) {
        self.cursor = self.cursor.saturating_sub(n);
        if self.cursor == 0 {
            self.unread = 0;
        }
    }

    /// Jump to the bottom (follow mode).
    pub fn reset_scroll(&mut self) {
        self.cursor = 0;
        self.unread = 0;
    }

    /// Clear all logs and reset scroll.
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
        self.unread = 0;
    }

    /// Render log text for a given area, clamping the cursor if needed.
    /// Returns the widget to render.
    pub fn format(&mut self, width: u16, height: u16, verbose: bool) -> Text<'static> {
        let (text, clamped) = self.buffer.to_text(width, height, self.cursor, verbose);
        self.cursor = clamped;
        text
    }

    pub fn unread(&self) -> usize {
        self.unread
    }
}
