use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::ui::Command;

#[derive(Default, Debug, Clone)]
pub struct ShellPrompt {
    input: String,
}

impl ShellPrompt {
    pub(super) fn on_key(&mut self, key: KeyEvent) -> Option<Command> {
        match key {
            KeyEvent {
                code: KeyCode::Char(c),
                modifiers: crossterm::event::KeyModifiers::CONTROL,
                ..
            } => match c {
                'c' => self.input.clear(),
                'd' if self.input.is_empty() => return Some(Command::EXIT),
                'l' if self.input.is_empty() => return Some(Command::CLEAR),
                _ => {}
            },
            KeyEvent {
                code: KeyCode::Char(c),
                ..
            } => self.input.push(c),
            KeyEvent {
                code: KeyCode::Backspace,
                ..
            } => {
                self.input.pop();
            }
            KeyEvent {
                code: KeyCode::Enter,
                ..
            } => {
                let command = self.input.parse::<Command>();
                match command {
                    Ok(_) => log::info!("$ {}", self.input),
                    Err(_) if !self.input.trim().is_empty() => log::error!("$ {}", self.input),
                    _ => {}
                }
                self.input.clear();
                return command.ok();
            }
            _ => {}
        }

        None
    }

    pub(super) fn render(
        &self,
        frame: &mut ::ratatui::Frame,
        area: ::ratatui::layout::Rect,
        color: Color,
    ) {
        let line = Line::from(vec![
            // This reads "operon$ ".
            Span::styled("operon$ ", Style::new().fg(color)),
            Span::raw(self.input.clone()),
            Span::raw("█"),
        ])
        .left_aligned();
        frame.render_widget(line, area);
    }
}
