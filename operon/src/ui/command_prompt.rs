use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::ui::Command;

#[derive(Default, Debug, Clone)]
pub struct CommandPrompt {
    input: String,
}

impl CommandPrompt {
    pub(super) fn on_key(&mut self, key: KeyEvent) -> Option<Command> {
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => self.input.clear(),
            (KeyModifiers::CONTROL, KeyCode::Char('d')) if self.input.is_empty() => {
                return Some(Command::Exit);
            }
            (KeyModifiers::CONTROL, KeyCode::Char('l')) if self.input.is_empty() => {
                return Some(Command::Clear);
            }
            (_, KeyCode::Char(c)) => self.input.push(c),
            (_, KeyCode::Backspace) => {
                self.input.pop();
            }
            (_, KeyCode::Enter) => {
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

    pub(super) fn to_line(&self, color: Color) -> Line<'static> {
        Line::from(vec![
            // This reads "operon$ ".
            Span::styled("operon$ ", Style::new().fg(color)),
            Span::raw(self.input.clone()),
            Span::raw("█"),
        ])
        .left_aligned()
    }
}
