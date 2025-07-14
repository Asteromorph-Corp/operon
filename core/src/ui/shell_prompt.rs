use clap::Parser;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

use crate::ui::{Action, PromptCommand};

#[derive(Default, Debug, Clone)]
pub struct ShellPrompt {
    input: String,
}

impl ShellPrompt {
    pub(super) fn on_key(&mut self, key: KeyEvent) -> Option<PromptCommand> {
        match key {
            KeyEvent {
                code: KeyCode::Char(c),
                modifiers: crossterm::event::KeyModifiers::CONTROL,
                ..
            } => match c {
                'c' => {
                    self.input.clear();
                    None
                }
                'd' => {
                    if self.input.is_empty() {
                        Some(PromptCommand {
                            action: Action::Exit,
                        })
                    } else {
                        None
                    }
                }
                'l' => {
                    if self.input.is_empty() {
                        Some(PromptCommand {
                            action: Action::Clear,
                        })
                    } else {
                        None
                    }
                }
                _ => None,
            },
            KeyEvent {
                code: KeyCode::Char(c),
                ..
            } => {
                self.input.push(c);
                None
            }
            KeyEvent {
                code: KeyCode::Backspace,
                ..
            } => {
                self.input.pop();
                None
            }
            KeyEvent {
                code: KeyCode::Enter,
                ..
            } => {
                let clap_input = String::from("operon ") + &self.input;
                let args = clap_input.split_whitespace();
                if args.clone().count() == 1 {
                    // No command entered, just return None
                    self.input.clear();
                    return None;
                }
                let command = match PromptCommand::try_parse_from(args) {
                    Ok(cmd) => {
                        log::info!("$ {}", self.input);
                        Some(cmd)
                    }
                    Err(_) => {
                        log::error!("$ {}", self.input);
                        None
                    }
                };
                self.input.clear();
                command
            }
            _ => None,
        }
    }

    pub(super) fn render(
        &self,
        frame: &mut ::ratatui::Frame,
        area: ::ratatui::layout::Rect,
        color: Color,
    ) {
        let line = Line::from(vec![
            // This reads "operon@<package.name>$ ".
            Span::styled("operon@ex1$ ", Style::new().fg(color)),
            Span::raw(self.input.clone()),
            Span::raw("█"),
        ])
        .left_aligned();
        frame.render_widget(line, area);
    }
}
