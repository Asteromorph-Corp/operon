use std::sync::Arc;

use crossterm::{
    event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers, MouseEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures::StreamExt;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::RwLock;

use crate::{
    operon::RunningState,
    scheduler::{ControlEvent, ControlEventSender, RecoveryState, RecoveryStateReceiver},
    ui::{Action, LogRecordReceiver, Progress, UiError, UiState, UiStateUpdate},
    utils::SplitFirstOwned,
};

const SEVENTY_SIX: u16 = 76;
const HELP_TEXT: &str = r#"Operon TUI.
Navigation keys:
    ^C                  Clear input.
    ^D                  Exit.
    ^L                  Clear logs.
    ^Up, ^Down          Scroll logs 1 line.
    Up, Down            Scroll logs 5 lines.
    PgUp, PgDn          Scroll logs 20 lines.
    Esc                 Show most recent logs.

Commands:
    run [OPTIONS]       Start a new run using the best available restoration (unless overridden by options).
        -f, --fresh         Start a fresh run, ignoring any existing data. Takes precedence over `rebuild`.
        -r, --rebuild       Rebuild the run from trusted data before starting.
    check               Check the consistency of the data from the last run.
    exit                Exit the UI.
    clear               Clear the log buffer.
    quit [OPTIONS]      Stop all jobs and exit the UI. Defaults to graceful shutdown.
        -f, --force         Force quit.
        -n, --no-exit       Don't exit the UI.
    pause [OPTIONS] [<JOB_TYPE>[ ...]]
                        Pause executing new jobs.
        -c, --cascade       Cascade the pause command to dependent jobs.
    resume [<JOB_TYPE>[ ...]]
                        Resume paused jobs.
    help                Print this help message."#;

/// The main UI loop that handles user input and updates the UI state.
pub struct UiLoop {
    state: Arc<RwLock<UiState>>,
    primary_ub: usize,
    log_rx: LogRecordReceiver,
    ctrl_tx: ControlEventSender,
    rec_rx: RecoveryStateReceiver,
}

impl UiLoop {
    /// Create a new UI loop with the given state, primary upper bound, log receiver,
    /// control event sender, and recovery state receiver.
    pub fn new(
        state: Arc<RwLock<UiState>>,
        primary_ub: usize,
        log_rx: LogRecordReceiver,
        ctrl_tx: ControlEventSender,
        rec_rx: RecoveryStateReceiver,
    ) -> Self {
        Self {
            state,
            primary_ub,
            log_rx,
            ctrl_tx,
            rec_rx,
        }
    }

    pub async fn run(mut self) -> Result<(), UiError> {
        enable_raw_mode()?;
        let mut stdout = ::std::io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;

        // Main loop for the UI.
        // Note: breaking this loop exits the UI, at least guard against `any_alive` before breaking.
        let mut events = EventStream::new();
        loop {
            {
                let guard = self.state.read().await;
                if !guard.any_alive() && guard.exit_on_finish {
                    break;
                }
            }
            tokio::select! {
                evt = events.next() => {
                    let Some(evt) = evt else {
                        // Stream closed, exit the UI.
                        break;
                    };
                    let command = match evt? {
                        Event::Key(KeyEvent {
                            code: KeyCode::Up,
                            modifiers: KeyModifiers::CONTROL,
                            ..
                        }) => {
                            let _ = self.state.write().await.cursor.saturating_add(1);
                            None
                        }
                        Event::Key(KeyEvent {
                            code: KeyCode::Down,
                            modifiers: KeyModifiers::CONTROL,
                            ..
                        }) => {
                            let mut state = self.state.write().await;
                            state.cursor = state.cursor.saturating_sub(1);
                            if state.cursor == 0 {
                                state.unread_logs = 0;
                            }
                            None
                        }
                        Event::Key(KeyEvent {
                            code: KeyCode::Up, ..
                        }) => {
                            let mut state = self.state.write().await;
                            state.cursor = state.cursor.saturating_add(5);
                            None
                        }
                        Event::Key(KeyEvent {
                            code: KeyCode::Down,
                            ..
                        }) => {
                            let mut state = self.state.write().await;
                            state.cursor = state.cursor.saturating_sub(5);
                            if state.cursor == 0 {
                                state.unread_logs = 0;
                            }
                            None
                        }
                        Event::Key(KeyEvent {
                            code: KeyCode::PageUp,
                            ..
                        }) => {
                            let mut state = self.state.write().await;
                            state.cursor = state.cursor.saturating_add(20);
                            None
                        }
                        Event::Key(KeyEvent {
                            code: KeyCode::PageDown,
                            ..
                        }) => {
                            let mut state = self.state.write().await;
                            state.cursor = state.cursor.saturating_sub(20);
                            if state.cursor == 0 {
                                state.unread_logs = 0;
                            }
                            None
                        }
                        Event::Key(KeyEvent {
                            code: KeyCode::Esc, ..
                        }) => {
                            let mut state = self.state.write().await;
                            state.cursor = 0;
                            state.unread_logs = 0;
                            None
                        }
                        Event::Key(key) => {
                            self.state.write().await.shell.on_key(key)
                        }
                        Event::Mouse(me) => match me.kind {
                            MouseEventKind::ScrollUp => {
                                let mut state = self.state.write().await;
                                state.cursor = state.cursor.saturating_add(5);
                                None
                            }
                            MouseEventKind::ScrollDown => {
                                let mut state = self.state.write().await;
                                state.cursor = state.cursor.saturating_sub(5);
                                if state.cursor == 0 {
                                    state.unread_logs = 0;
                                }
                                None
                            }
                            _ => None,
                        },
                        _ => None,
                    };

                    // Fetch the recovery state.
                    let rec_state = *self.rec_rx.borrow();
                    if rec_state == RecoveryState::Error {
                        log::error!("Error in scheduler startup, exiting UI.");
                        break;
                    }
                    // Lock and clone current UiState for command execution.
                    let exec_snapshot = self.state.read().await.clone();
                    let overall_state_snapshot = exec_snapshot.overall_state();

                    let Some(command) = command else {
                        // No command, just continue to the next event.
                        continue;
                    };

                    // Execute the command if any.
                    match command.action {
                        Action::Run { fresh, rebuild } => match exec_snapshot.last_control_event {
                            ControlEvent::Start => match rec_state {
                                RecoveryState::Unknown => {
                                    log::warn!("Scheduler was not initialized yet.")
                                }
                                RecoveryState::Fresh
                                | RecoveryState::Finished => {
                                    if rebuild {
                                        log::error!("Cannot rebuild.")
                                    } else {
                                        self.ctrl_tx.send(ControlEvent::clean_run(self.primary_ub))?;
                                        self.state.write().await.update_ui_state(ControlEvent::clean_run(self.primary_ub))?;
                                    }
                                }
                                RecoveryState::AbortedUnchecked => {
                                    if rebuild {
                                        log::error!("Cannot rebuild before checking for consistency.")
                                    } else {
                                        self.ctrl_tx.send(ControlEvent::clean_run(self.primary_ub))?;
                                        self.state.write().await.update_ui_state(ControlEvent::clean_run(self.primary_ub))?;
                                    }
                                }
                                RecoveryState::AbortedChecked => {
                                    if fresh {
                                        log::info!("Starting a fresh run, ignoring previous data.");
                                        self.ctrl_tx.send(ControlEvent::clean_run(self.primary_ub))?;
                                        self.state.write().await.update_ui_state(ControlEvent::clean_run(self.primary_ub))?;
                                    } else {
                                        log::info!("Rebuilding the run from trusted data.");
                                        self.ctrl_tx.send(ControlEvent::rebuild_run(self.primary_ub))?;
                                        self.state.write().await.update_ui_state(ControlEvent::rebuild_run(self.primary_ub))?;
                                    }
                                }
                                RecoveryState::GracefullyStopped => {
                                    if fresh {
                                        log::info!("Starting a fresh run, ignoring previous data.");
                                        self.ctrl_tx.send(ControlEvent::clean_run(self.primary_ub))?;
                                        self.state.write().await.update_ui_state(ControlEvent::clean_run(self.primary_ub))?;
                                    } else if rebuild {
                                        log::info!("Rebuilding the run from trusted data.");
                                        self.ctrl_tx.send(ControlEvent::rebuild_run(self.primary_ub))?;
                                        self.state.write().await.update_ui_state(ControlEvent::rebuild_run(self.primary_ub))?;
                                    } else {
                                        log::info!("Continuing the last run.");
                                        self.ctrl_tx.send(ControlEvent::restore_run(self.primary_ub))?;
                                        self.state.write().await.update_ui_state(ControlEvent::restore_run(self.primary_ub))?;
                                    }
                                }
                                _ => unreachable!(),
                            },
                            ControlEvent::Check { .. } => match rec_state {
                                RecoveryState::MissingData => {
                                    if rebuild {
                                        log::error!("Cannot rebuild.");
                                        continue;
                                    }
                                    self.ctrl_tx.send(ControlEvent::clean_run(self.primary_ub))?;
                                    self.state.write().await.update_ui_state(ControlEvent::clean_run(self.primary_ub))?;
                                }
                                RecoveryState::AbortedChecked => {
                                    if fresh {
                                        log::info!("Starting a fresh run, ignoring previous data.");
                                        self.ctrl_tx.send(ControlEvent::clean_run(self.primary_ub))?;
                                        self.state.write().await.update_ui_state(ControlEvent::clean_run(self.primary_ub))?;
                                    } else {
                                        log::info!("Rebuilding the run from trusted data.");
                                        self.ctrl_tx.send(ControlEvent::rebuild_run(self.primary_ub))?;
                                        self.state.write().await.update_ui_state(ControlEvent::rebuild_run(self.primary_ub))?;
                                    }
                                }
                                RecoveryState::GracefullyStoppedChecked => {
                                    if fresh {
                                        log::info!("Starting a fresh run, ignoring previous data.");
                                        self.ctrl_tx.send(ControlEvent::clean_run(self.primary_ub))?;
                                        self.state.write().await.update_ui_state(ControlEvent::clean_run(self.primary_ub))?;
                                    } else if rebuild {
                                        log::info!("Rebuilding the run from trusted data.");
                                        self.ctrl_tx.send(ControlEvent::rebuild_run(self.primary_ub))?;
                                        self.state.write().await.update_ui_state(ControlEvent::rebuild_run(self.primary_ub))?;
                                    } else {
                                        log::info!("Continuing the last run.");
                                        self.ctrl_tx.send(ControlEvent::restore_run(self.primary_ub))?;
                                        self.state.write().await.update_ui_state(ControlEvent::restore_run(self.primary_ub))?;
                                    }
                                }
                                _ => {
                                    log::warn!("Please wait until the check is finished.");
                                }
                            },
                            _ => {
                                log::warn!(
                                    "Already run. Use `exit` or `quit` to terminate the current session before starting a new run."
                                );
                            }
                        },
                        Action::Check => match exec_snapshot.last_control_event {
                            ControlEvent::Check { .. } => {
                                log::warn!("Already run a check.");
                            }
                            ControlEvent::Start => match rec_state {
                                RecoveryState::AbortedUnchecked
                                | RecoveryState::GracefullyStopped => {
                                    log::info!("Starting a consistency check of the remaining data.");
                                    self.ctrl_tx.send(ControlEvent::check(self.primary_ub))?;
                                    self.state.write().await.update_ui_state(ControlEvent::check(self.primary_ub))?;
                                }
                                _ => {
                                    log::warn!(
                                        "Checks are only available when the previous run was aborted or gracefully stopped."
                                    );
                                }
                            },
                            _ => {
                                log::warn!("Cannot check after the run has already started.");
                            }
                        },
                        Action::Exit => match exec_snapshot.last_control_event {
                            ControlEvent::Start | ControlEvent::Check { .. } => {
                                // We didn't start any jobs, so we can exit immediately.
                                self.ctrl_tx.send(ControlEvent::Abort)?;
                                break;
                            }
                            ControlEvent::Abort | ControlEvent::GracefulStop
                                if exec_snapshot.any_alive() =>
                            {
                                log::warn!(
                                    "Please wait until the current jobs are stopped before exiting."
                                );
                            }
                            _ => match overall_state_snapshot {
                                RunningState::Finished
                                | RunningState::Stopped
                                | RunningState::Error => {
                                    // `quit` first, then exit.
                                    if exec_snapshot.any_alive() {
                                        self.ctrl_tx.send(ControlEvent::Abort)?;
                                        self.state.write().await.update_ui_state(ControlEvent::Abort)?;
                                        self.state.write().await.update_ui_state(
                                            UiStateUpdate::ExitOnFinish(true)
                                        )?;
                                    } else {
                                        break;
                                    }
                                }
                                RunningState::Running | RunningState::Paused => {
                                    log::warn!(
                                        "Cannot exit while jobs are running or paused. \
                                        Use `quit` for a graceful stop, or `quit --force` to abort all jobs."
                                    );
                                }
                            },
                        },
                        Action::Clear => {
                            let mut guard = self.state.write().await;
                            guard.log_buffer.clear();
                            guard.cursor = 0;
                            guard.unread_logs = 0;
                        }
                        Action::Quit { force, no_exit } => match exec_snapshot.last_control_event {
                            ControlEvent::Start | ControlEvent::Check { .. } => {
                                if no_exit {
                                    log::warn!("Cannot quit before the run has started.");
                                    continue;
                                }
                                self.ctrl_tx.send(ControlEvent::Abort)?;
                                break;
                            }
                            ControlEvent::Abort if exec_snapshot.any_alive() => {
                                log::warn!("Already processing an abort.");
                            }
                            ControlEvent::GracefulStop if exec_snapshot.any_alive() => {
                                if !force {
                                    log::warn!("Already processing a graceful stop.");
                                    continue;
                                }

                                log::warn!("Already processing a graceful stop, but force quit requested.");
                                self.ctrl_tx.send(ControlEvent::Abort)?;
                                self.state.write().await.update_ui_state(ControlEvent::Abort)?;
                            }
                            _ => match overall_state_snapshot {
                                RunningState::Finished | RunningState::Stopped => {
                                    if !exec_snapshot.any_alive() {
                                        if !no_exit { break; }
                                        log::warn!("Nothing to quit.");
                                        continue;
                                    }

                                    self.ctrl_tx.send(ControlEvent::Abort)?;
                                    self.state.write().await.update_ui_state(ControlEvent::Abort)?;
                                    self.state.write().await.update_ui_state(
                                        UiStateUpdate::ExitOnFinish(!no_exit),
                                    )?;
                                }
                                RunningState::Error => {
                                    if !exec_snapshot.any_alive() {
                                        if !no_exit { break; }
                                        log::warn!("Nothing to quit.");
                                        continue;
                                    }

                                    if !force {
                                        log::warn!(
                                            "Cannot gracefully stop due to previous errors. \
                                            Defaulting to a force quit."
                                        );
                                    }
                                    self.ctrl_tx.send(ControlEvent::Abort)?;
                                    self.state.write().await.update_ui_state(ControlEvent::Abort)?;
                                    // We *don't* exit here (even without the no-exit flag),
                                    // because the user should be able to inspect the logs.
                                    if exec_snapshot.exit_on_finish {
                                        self.state.write().await.update_ui_state(
                                            UiStateUpdate::ExitOnFinish(false),
                                        )?;
                                    }
                                }
                                RunningState::Paused | RunningState::Running => {
                                    if force {
                                        // Send an `Abort` event to all schedulers
                                        log::info!("Sent abort request, stopping immediately...");
                                        self.ctrl_tx.send(ControlEvent::Abort)?;
                                        self.state.write().await.update_ui_state(ControlEvent::Abort)?;
                                    } else {
                                        // Send a `GracefulStop` command to all schedulers
                                        log::info!("Sent stop request, stopping gracefully...");
                                        self.ctrl_tx.send(ControlEvent::GracefulStop)?;
                                        self.state.write().await.update_ui_state(ControlEvent::GracefulStop)?;
                                    }
                                    self.state.write().await.update_ui_state(
                                        UiStateUpdate::ExitOnFinish(!no_exit),
                                    )?;
                                }
                            },
                        },
                        Action::Pause { targets, cascade } => match exec_snapshot.last_control_event {
                            ControlEvent::Start | ControlEvent::Check { .. } => {
                                log::warn!("Cannot pause before the run has started.");
                            }
                            ControlEvent::Abort | ControlEvent::GracefulStop
                                if exec_snapshot.any_alive() =>
                            {
                                log::warn!("Cannot pause while stopping.");
                            }
                            _ => {
                                if !exec_snapshot
                                    .state_iter()
                                    .any(|s| s == RunningState::Running)
                                {
                                    log::warn!(
                                        "No running jobs to pause, did you mean to `exit` or `quit` instead?"
                                    );
                                    continue;
                                }
                                self.ctrl_tx.send(ControlEvent::pause(targets.clone(), cascade))?;
                                self.state.write().await.update_ui_state(ControlEvent::pause(targets, cascade))?;
                            }
                        }
                        Action::Resume { targets } => match exec_snapshot.last_control_event {
                            ControlEvent::Start | ControlEvent::Check { .. } => {
                                log::warn!("Cannot resume before the run has started.");
                            }
                            ControlEvent::Abort | ControlEvent::GracefulStop
                                if exec_snapshot.any_alive() =>
                            {
                                log::warn!("Cannot resume while stopping.");
                            }
                            _ => {
                                if !exec_snapshot
                                    .state_iter()
                                    .any(|s| s == RunningState::Paused)
                                {
                                    log::warn!("No paused jobs to resume.");
                                    continue;
                                }
                                self.ctrl_tx.send(ControlEvent::resume(targets.clone()))?;
                                self.state.write().await.update_ui_state(ControlEvent::resume(targets.clone()))?;
                            }
                        },
                        Action::Help => log::info!("{HELP_TEXT}"),
                        // _ => log::warn!("Command not yet implemented: {command:?}"),
                    }
                }

                _ = ::tokio::time::sleep(::std::time::Duration::from_millis(10)) => {
                    // Drain the log channel before drawing the UI.
                    loop {
                        match self.log_rx.try_recv() {
                            Ok(record) => self.state.write().await.update_ui_state(
                                UiStateUpdate::NewLog(
                                    record,
                                    terminal.size()?.width,
                                ),
                            )?,
                            // Skip fallen-behind logs
                            Err(::tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => continue,
                            // Drained all logs
                            Err(::tokio::sync::broadcast::error::TryRecvError::Empty) => break,
                            // Channel unexpectedly closed
                            Err(e) => return Err(e.into()),
                        }
                    }

                    // Draw the UI state after the command execution
                    self.draw(&mut terminal).await?;
                }
            }
        }

        // Cleanup:
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;
        Ok(())
    }

    async fn draw(&self, terminal: &mut Terminal<impl Backend>) -> Result<(), UiError> {
        let draw_snapshot = self.state.read().await.clone();
        // Draw the major pane(s)
        let mut cursor = draw_snapshot.cursor;
        terminal.draw(|frame| {
            let [
                progress_head,
                progress_area,
                progress_foot,
                logs_head,
                logs_area,
                logs_foot,
                input_area,
            ] = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(6),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Fill(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .areas(frame.area());
            frame.render_widget(
                Block::new().borders(Borders::TOP).title(
                    Line::from(vec![
                        Span::raw("│ "),
                        Span::raw("Progress").italic(),
                        Span::raw(" ├"),
                    ])
                    .left_aligned(),
                ),
                progress_head,
            );
            let constraints =
                std::iter::repeat_n(Constraint::Fill(1), draw_snapshot.progress.len() + 1)
                    .collect::<Vec<_>>();
            let (progress_description, progress_bars) = Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints)
                .split(progress_area)
                .iter()
                .cloned()
                .split_first_owned()
                .expect("Progress area must have at least one bar");

            let max_len = draw_snapshot
                .progress
                .keys()
                .map(|name| u16::try_from(name.len()).expect("Progress name too long"))
                .max()
                .unwrap_or(0);

            frame.render_widget(
                Line::from(if progress_description.width >= SEVENTY_SIX + max_len {
                    vec![
                        Span::raw(format!("{:4}", "")),
                        Span::raw("job").underlined(),
                        Span::raw("   "),
                        Span::raw("done").underlined(),
                        Span::raw(" "),
                        Span::raw("ready").underlined(),
                        Span::raw("  "),
                        Span::raw("wait").underlined(),
                        Span::raw("   Colors: ").dark_gray().italic(),
                        Span::raw("Finished").green().italic(),
                        Span::raw(" | ").dark_gray(),
                        Span::raw("Running").cyan().italic(),
                        Span::raw(" | ").dark_gray(),
                        Span::raw("Paused").yellow().italic(),
                        Span::raw(" | ").dark_gray(),
                        Span::raw("Error").red().italic(),
                        Span::raw(" | ").dark_gray(),
                        Span::raw("Stopped").dark_gray().italic(),
                    ]
                } else {
                    vec![
                        Span::raw(format!("{:4}", "")),
                        Span::raw("job").underlined(),
                        Span::raw("   "),
                        Span::raw("done").underlined(),
                        Span::raw(" "),
                        Span::raw("ready").underlined(),
                        Span::raw("  "),
                        Span::raw("wait").underlined(),
                    ]
                })
                .left_aligned(),
                progress_description,
            );
            draw_snapshot
                .progress
                .iter()
                .zip(progress_bars)
                .for_each(|((name, progress), bar)| {
                    draw_progress_gauge(frame, bar, name, progress);
                });
            frame.render_widget(Block::new().borders(Borders::BOTTOM), progress_foot);
            frame.render_widget(
                Block::new().borders(Borders::TOP).title(
                    Line::from(vec![
                        Span::raw("│ "),
                        Span::raw("Logs").italic(),
                        Span::raw(" ├"),
                    ])
                    .left_aligned(),
                ),
                logs_head,
            );
            let (logs_widget, new_cursor) = draw_snapshot.log_buffer.to_text(
                logs_area.width,
                logs_area.height,
                draw_snapshot.cursor,
            );
            cursor = new_cursor;
            frame.render_widget(logs_widget, logs_area);
            frame.render_widget(
                Block::new().borders(Borders::BOTTOM).title(
                    Line::from(if draw_snapshot.unread_logs == 0 {
                        vec![]
                    } else {
                        vec![
                            Span::raw("┤ "),
                            Span::raw(format!(
                                "{} unread, Esc to follow",
                                draw_snapshot.unread_logs
                            ))
                            .italic(),
                            Span::raw(" │"),
                        ]
                    })
                    .right_aligned(),
                ),
                logs_foot,
            );
            draw_snapshot
                .shell
                .render(frame, input_area, draw_snapshot.overall_state().color());
        })?;
        // Update the cursor position if it was clipped
        if cursor != draw_snapshot.cursor {
            self.state
                .write()
                .await
                .update_ui_state(UiStateUpdate::SetCursor(cursor))?;
        }
        Ok(())
    }
}

/// Formats the count for display in the progress gauge.
fn five_format(count: i64) -> String {
    debug_assert!(count >= 0, "Count must be non-negative");
    fn intdiv_truncate_down(raw: i64, divis_power: u32, digits_under_point: u32) -> f64 {
        let divisor = 10i64.pow(divis_power.saturating_sub(digits_under_point));
        let divided = raw / divisor;
        let factor = 10f64.powi(digits_under_point as i32);
        divided as f64 / factor
    }
    let raw_len = count.to_string().len();
    match raw_len {
        i if i < 5 => format!("{count:>5}"),
        5 => format!("{:.1}K", intdiv_truncate_down(count, 3, 1)),
        6 => format!(" {}K", intdiv_truncate_down(count, 3, 0) as i64),
        7 => format!("{:.2}M", intdiv_truncate_down(count, 6, 2)),
        8 => format!("{:.1}M", intdiv_truncate_down(count, 6, 1)),
        9 => format!(" {}M", intdiv_truncate_down(count, 6, 0) as i64),
        10 => format!("{:.2}B", intdiv_truncate_down(count, 9, 2)),
        11 => format!("{:.1}B", intdiv_truncate_down(count, 9, 1)),
        12 => format!(" {}B", intdiv_truncate_down(count, 9, 0) as i64),
        13 => format!("{:.2}T", intdiv_truncate_down(count, 12, 2)),
        14 => format!("{:.1}T", intdiv_truncate_down(count, 12, 1)),
        15 => format!(" {}T", intdiv_truncate_down(count, 12, 0) as i64),
        16 => format!("{:.1}Qa", intdiv_truncate_down(count, 15, 1)),
        17 | 18 => format!("{:>3}Qa", intdiv_truncate_down(count, 15, 0) as i64),
        19 => format!("{:.1}Qi", intdiv_truncate_down(count, 18, 1)),
        _ => unreachable!(),
    }
}

/// Draws a progress gauge with a text label and a manual gauge.
fn draw_progress_gauge(frame: &mut Frame<'_>, area: Rect, name: &str, progress: &Progress) {
    // Text area: "epsilon [ done/queue/ wait] "
    let text_length = 21 + 7; // 7 stands for the longest job name length
    let horizontal = Layout::horizontal([
        Constraint::Length(text_length),
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ]);
    let [text_area, gauge_left, gauge_area, gauge_right] = horizontal.areas(area);
    let (done, queued, waiting) = (progress.0, progress.1, progress.2);
    frame.render_widget(
        Line::from(vec![
            Span::styled(format!("{name:>7}"), Style::new().fg(progress.3.color())),
            Span::raw(format!(
                " [{}/{}/{}] ",
                five_format(done),
                five_format(queued),
                five_format(waiting)
            )),
        ]),
        text_area,
    );

    // Gauge area: manual gauge with Span, surrounded by borders
    let gauge_length = gauge_area.width;
    let done_length = if done + queued + waiting > 0 {
        ((done as f64 / (done + queued + waiting) as f64) * gauge_length as f64).round() as u16
    } else {
        0
    };
    let queued_length = if done + queued + waiting > 0 {
        ((queued as f64 / (done + queued + waiting) as f64) * gauge_length as f64).round() as u16
    } else {
        0
    };
    let waiting_length = gauge_length.saturating_sub(done_length + queued_length);
    let [done_area, queued_area, waiting_area] = Layout::horizontal([
        Constraint::Length(done_length),
        Constraint::Length(queued_length),
        Constraint::Fill(1),
    ])
    .areas(gauge_area);
    let done_span = Span::styled(
        "█".repeat(done_length as usize),
        Style::default().fg(progress.3.color()),
    );
    let queued_span = Span::styled(
        "░".repeat(queued_length as usize),
        Style::default().fg(progress.3.color()),
    );
    let waiting_span = Span::styled(" ".repeat(waiting_length as usize), Style::default());
    frame.render_widget(
        Block::new()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(Color::White)),
        gauge_left,
    );
    frame.render_widget(Paragraph::new(done_span), done_area);
    frame.render_widget(Paragraph::new(queued_span), queued_area);
    frame.render_widget(Paragraph::new(waiting_span), waiting_area);
    frame.render_widget(
        Block::new()
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(Color::White)),
        gauge_right,
    );
}
