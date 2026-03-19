use std::sync::Arc;

use crossterm::event::{Event, EventStream, KeyCode, KeyModifiers, MouseEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use futures::StreamExt;
use ratatui::prelude::*;
use ratatui::widgets::*;
use tokio::sync::RwLock;

use crate::scheduler::{
    ControlEvent, ControlEventSender, ExecutionState, RecoveryState, RecoveryStateReceiver,
    SchedulerStateReceiver,
};
use crate::ui::{
    Command, CommandPrompt, LogRecordReceiver, LogView, Progress, UiError, UiMode, UiOptions,
    UiState,
};
use crate::utils::SplitFirstOwned;

const SEVENTY_SIX: u16 = 76;
const HELP_TEXT: &str = r#"Operon TUI.
Navigation keys:
    Ctrl+C              Clear input.
    Ctrl+D              Exit.
    Ctrl+L              Clear logs.
    Left, Right         Scroll progress bars.
    Alt+Up, Alt+Down    Scroll logs 1 line.
    Up, Down            Scroll logs 5 lines.
    PgUp, PgDn          Scroll logs 20 lines.
    Esc                 Show most recent logs.

Commands:
    run [OPTIONS]       Start a new run using the best available restoration (unless overridden by options).
        -f, --fresh         Start a fresh run, ignoring any existing data. Takes precedence over `rebuild`.
        -r, --rebuild       Rebuild the run from trusted data before starting.
    check [OPTIONS]     Check the consistency of the data from the last run.
        -m, --mode [MODE]   Mode of the consistency check. Defaults to "quick". Options:
            trust-all           Assume all data is trustworthy, skipping checks.
            metadata-only       Check only metadata consistency.
            quick               Perform a metadata check plus data validation only at boundaries.
            exhaustive          Perform a full consistency check of all data. (Can be very slow.)
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
    mode: UiMode,
    progress_cursor: u16,
    logs: LogView,
    prompt: CommandPrompt,
    log_rx: LogRecordReceiver,
    ctrl_tx: ControlEventSender,
    rec_rx: RecoveryStateReceiver,
    sched_rx: SchedulerStateReceiver,
    exit_on_finish: bool,
}

impl UiLoop {
    /// Create a new UI loop with the given state, primary upper bound, log receiver,
    /// control event sender, and recovery state receiver.
    pub fn new(
        state: Arc<RwLock<UiState>>,
        log_rx: LogRecordReceiver,
        ctrl_tx: ControlEventSender,
        rec_rx: RecoveryStateReceiver,
        sched_rx: SchedulerStateReceiver,
        options: UiOptions,
    ) -> Self {
        Self {
            state,
            mode: options.mode,
            progress_cursor: 0u16,
            logs: LogView::new(options.log_buffer_size),
            prompt: CommandPrompt::default(),
            log_rx,
            ctrl_tx,
            rec_rx,
            sched_rx,
            exit_on_finish: false,
        }
    }

    pub async fn run(self) -> Result<(), UiError> {
        match self.mode {
            UiMode::Interactive => self.run_interactive().await,
            UiMode::Headless => self.run_headless().await,
        }
    }

    pub async fn run_interactive(mut self) -> Result<(), UiError> {
        enable_raw_mode()?;
        let mut stdout = ::std::io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;

        // Main loop for the UI.
        // Note: breaking this loop exits the UI, at least guard against `any_alive` before
        // breaking.
        let mut events = EventStream::new();
        loop {
            {
                let guard = self.state.read().await;
                if !guard.any_alive() && self.exit_on_finish {
                    break;
                }
            }
            tokio::select! {
                evt = events.next() => {
                    let Some(evt) = evt else {
                        // Stream closed, exit the UI.
                        break;
                    };
                    let command = self.handle_event(evt?);

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
                    match command {
                        Command::Run { fresh, rebuild }  => match exec_snapshot.last_control_event {
                            // DONE: Handle `Command::Run` in `IdleState`.
                            // TODO: Handle `ControlEvent::Run` in scheduler.
                            ControlEvent::Start => match rec_state {
                                RecoveryState::Unknown => log::warn!("Scheduler was not initialized yet."),
                                RecoveryState::Fresh | RecoveryState::Finished => {
                                    if rebuild {
                                        log::error!("Cannot rebuild.")
                                    } else {
                                        self.ctrl_tx.send(ControlEvent::CleanRun)?;
                                        self.state.write().await.update_ui_state(ControlEvent::CleanRun)?;
                                    }
                                }
                                RecoveryState::AbortedUnchecked => {
                                    if rebuild {
                                        log::error!("Cannot rebuild before checking for consistency.")
                                    } else {
                                        self.ctrl_tx.send(ControlEvent::CleanRun)?;
                                        self.state.write().await.update_ui_state(ControlEvent::CleanRun)?;
                                    }
                                }
                                RecoveryState::AbortedChecked => {
                                    if fresh {
                                        log::info!("Starting a fresh run, ignoring previous data.");
                                        self.ctrl_tx.send(ControlEvent::CleanRun)?;
                                        self.state.write().await.update_ui_state(ControlEvent::CleanRun)?;
                                    } else {
                                        log::info!("Rebuilding the run from trusted data.");
                                        self.ctrl_tx.send(ControlEvent::RebuildRun)?;
                                        self.state.write().await.update_ui_state(ControlEvent::RebuildRun)?;
                                    }
                                }
                                RecoveryState::GracefullyStopped => {
                                    if fresh {
                                        log::info!("Starting a fresh run, ignoring previous data.");
                                        self.ctrl_tx.send(ControlEvent::CleanRun)?;
                                        self.state.write().await.update_ui_state(ControlEvent::CleanRun)?;
                                    } else if rebuild {
                                        log::info!("Rebuilding the run from trusted data.");
                                        self.ctrl_tx.send(ControlEvent::RebuildRun)?;
                                        self.state.write().await.update_ui_state(ControlEvent::RebuildRun)?;
                                    } else {
                                        log::info!("Continuing the last run.");
                                        self.ctrl_tx.send(ControlEvent::RestoreRun)?;
                                        self.state.write().await.update_ui_state(ControlEvent::RestoreRun)?;
                                    }
                                }
                                _ => unreachable!(),
                            },
                            // DONE: Handle `Command::Run` from `IdleState`.
                            // TODO: Handle `ControlEvent::Run` in scheduler.
                            ControlEvent::Check { .. } => match rec_state {
                                RecoveryState::MissingData => {
                                    if rebuild {
                                        log::error!("Cannot rebuild.");
                                        continue;
                                    }
                                    self.ctrl_tx.send(ControlEvent::CleanRun)?;
                                    self.state.write().await.update_ui_state(ControlEvent::CleanRun)?;
                                }
                                RecoveryState::AbortedChecked => {
                                    if fresh {
                                        log::info!("Starting a fresh run, ignoring previous data.");
                                        self.ctrl_tx.send(ControlEvent::CleanRun)?;
                                        self.state.write().await.update_ui_state(ControlEvent::CleanRun)?;
                                    } else {
                                        log::info!("Rebuilding the run from trusted data.");
                                        self.ctrl_tx.send(ControlEvent::RebuildRun)?;
                                        self.state.write().await.update_ui_state(ControlEvent::RebuildRun)?;
                                    }
                                }
                                RecoveryState::GracefullyStoppedChecked => {
                                    if fresh {
                                        log::info!("Starting a fresh run, ignoring previous data.");
                                        self.ctrl_tx.send(ControlEvent::CleanRun)?;
                                        self.state.write().await.update_ui_state(ControlEvent::CleanRun)?;
                                    } else if rebuild {
                                        log::info!("Rebuilding the run from trusted data.");
                                        self.ctrl_tx.send(ControlEvent::RebuildRun)?;
                                        self.state.write().await.update_ui_state(ControlEvent::RebuildRun)?;
                                    } else {
                                        log::info!("Continuing the last run.");
                                        self.ctrl_tx.send(ControlEvent::RestoreRun)?;
                                        self.state.write().await.update_ui_state(ControlEvent::RestoreRun)?;
                                    }
                                }
                                RecoveryState::FinishedChecked => {
                                    if fresh {
                                        log::info!("Starting a fresh run, ignoring previous data.");
                                        self.ctrl_tx.send(ControlEvent::CleanRun)?;
                                        self.state.write().await.update_ui_state(ControlEvent::CleanRun)?;
                                    } else {
                                        log::info!("Continuing the last run.");
                                        self.ctrl_tx.send(ControlEvent::RestoreRun)?;
                                        self.state.write().await.update_ui_state(ControlEvent::RestoreRun)?;
                                    }
                                }
                                _ => {
                                    log::warn!("Please wait until the check is finished.");
                                }
                            },
                            // TODO: Reject `Command::Run` in other UI states.
                            _ => {
                                log::warn!(
                                    "Already run. Use `exit` or `quit` to terminate the current session before starting a new run."
                                );
                            }
                        },
                        Command::Check { mode } => match exec_snapshot.last_control_event {
                            // DONE: Reject `Command::Check` in `IdleState` if already checked.
                            ControlEvent::Check { .. } => {
                                log::warn!("Already run a check.");
                            }
                            // DONE: Handle `Command::Check` in `IdleState`.
                            // TODO: Reject `ControlEvent::Check` in scheduler if not appropriate.
                            ControlEvent::Start => match rec_state {
                                RecoveryState::AbortedUnchecked
                                | RecoveryState::GracefullyStopped | RecoveryState::Finished => {
                                    log::info!("Starting a consistency check of the remaining data.");
                                    self.ctrl_tx.send(ControlEvent::Check {mode})?;
                                    self.state.write().await.update_ui_state(ControlEvent::Check {mode})?;
                                }
                                _ => {
                                    log::warn!(
                                        "Checks are only available when the previous run was aborted or gracefully stopped."
                                    );
                                }
                            },
                            // TODO: Reject `Command::Check` in other UI states.
                            _ => {
                                log::warn!("Cannot check after the run has already started.");
                            }
                        },
                        Command::Exit => match exec_snapshot.last_control_event {
                            // DONE: Handle `Command::Exit` in `IdleState`.
                            ControlEvent::Start | ControlEvent::Check { .. } => {
                                // We didn't start any jobs, so we can exit immediately.
                                self.ctrl_tx.send(ControlEvent::Abort)?;
                                break;
                            }
                            // TODO: Handle `Command::Exit` in other UI states.
                            ControlEvent::Abort | ControlEvent::GracefulStop
                                if exec_snapshot.any_alive() =>
                            {
                                log::warn!(
                                    "Please wait until the current jobs are stopped before exiting."
                                );
                            }
                            // TODO: Handle `Command::Exit` in other UI states.
                            _ => match overall_state_snapshot {
                                ExecutionState::Finished
                                | ExecutionState::Stopped
                                | ExecutionState::Error => {
                                    // `quit` first, then exit.
                                    if exec_snapshot.any_alive() {
                                        self.ctrl_tx.send(ControlEvent::Abort)?;
                                        self.state.write().await.update_ui_state(ControlEvent::Abort)?;
                                        self.exit_on_finish = true;
                                    } else {
                                        break;
                                    }
                                }
                                ExecutionState::Running | ExecutionState::Paused => {
                                    log::warn!(
                                        "Cannot exit while jobs are running or paused. \
                                        Use `quit` for a graceful stop, or `quit --force` to abort all jobs."
                                    );
                                }
                            },
                        },
                        Command::Quit { force, no_exit } => match exec_snapshot.last_control_event {
                            // DONE: Handle `Command::Quit` in `IdleState`.
                            ControlEvent::Start | ControlEvent::Check { .. } => {
                                if no_exit {
                                    log::warn!("Cannot quit before the run has started.");
                                    continue;
                                }
                                self.ctrl_tx.send(ControlEvent::Abort)?;
                                break;
                            }
                            // TODO: Handle `Command::Quit` in other UI states.
                            ControlEvent::Abort if exec_snapshot.any_alive() => {
                                log::warn!("Already processing an abort.");
                            }
                            // TODO: Handle `Command::Quit` in other UI states.
                            ControlEvent::GracefulStop if exec_snapshot.any_alive() => {
                                if !force {
                                    log::warn!("Already processing a graceful stop.");
                                    continue;
                                }

                                log::warn!("Already processing a graceful stop, but force quit requested.");
                                self.ctrl_tx.send(ControlEvent::Abort)?;
                                self.state.write().await.update_ui_state(ControlEvent::Abort)?;
                            }
                            // TODO: Handle `Command::Quit` in other UI states.
                            _ => match overall_state_snapshot {
                                ExecutionState::Finished | ExecutionState::Stopped => {
                                    if !exec_snapshot.any_alive() {
                                        if !no_exit { break; }
                                        log::warn!("Nothing to quit.");
                                        continue;
                                    }

                                    self.ctrl_tx.send(ControlEvent::Abort)?;
                                    self.state.write().await.update_ui_state(ControlEvent::Abort)?;
                                    self.exit_on_finish = !no_exit;
                                }
                                ExecutionState::Error => {
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
                                    self.exit_on_finish = false;
                                }
                                ExecutionState::Paused | ExecutionState::Running => {
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
                                    self.exit_on_finish = !no_exit;
                                }
                            },
                        },
                        Command::Pause { targets, cascade } => match exec_snapshot.last_control_event {
                            // DONE: Handle `Command::Pause` in `IdleState`.
                            ControlEvent::Start | ControlEvent::Check { .. } => {
                                log::warn!("Cannot pause before the run has started.");
                            }
                            // TODO: Handle `Command::Pause` in other UI states.
                            ControlEvent::Abort | ControlEvent::GracefulStop
                                if exec_snapshot.any_alive() =>
                            {
                                log::warn!("Cannot pause while stopping.");
                            }
                            // TODO: Handle `Command::Pause` in other UI states.
                            _ => {
                                if !exec_snapshot
                                    .state_iter()
                                    .any(|s| s == ExecutionState::Running)
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
                        Command::Resume { targets } => match exec_snapshot.last_control_event {
                            // DONE: Handle `Command::Resume` in `IdleState`.
                            ControlEvent::Start | ControlEvent::Check { .. } => {
                                log::warn!("Cannot resume before the run has started.");
                            }
                            // TODO: Handle `Command::Resume` in other UI states.
                            ControlEvent::Abort | ControlEvent::GracefulStop
                                if exec_snapshot.any_alive() =>
                            {
                                log::warn!("Cannot resume while stopping.");
                            }
                            // TODO: Handle `Command::Resume` in other UI states.
                            _ => {
                                if !exec_snapshot
                                    .state_iter()
                                    .any(|s| s == ExecutionState::Paused)
                                {
                                    log::warn!("No paused jobs to resume.");
                                    continue;
                                }
                                self.ctrl_tx.send(ControlEvent::resume(targets.clone()))?;
                                self.state.write().await.update_ui_state(ControlEvent::resume(targets.clone()))?;
                            }
                        },
                        Command::Clear => self.logs.clear(),
                        Command::Help => log::info!("{HELP_TEXT}"),
                        // _ => log::warn!("Command not yet implemented: {command:?}"),
                    }
                }

                _ = ::tokio::time::sleep(::std::time::Duration::from_millis(10)) => {
                    // Drain the log channel before drawing the UI.
                    let width = terminal.size()?.width;
                    loop {
                        match self.log_rx.try_recv() {
                            Ok(record) => self.logs.push(record, width),
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

    pub async fn run_headless(mut self) -> Result<(), UiError> {
        // Main loop for the headless UI.
        // Recovery is disabled for this mode,
        // so we always run fresh off the bat and wait
        // until everything finishes or something errors.
        self.ctrl_tx.send(ControlEvent::CleanRun)?;
        self.state
            .write()
            .await
            .update_ui_state(ControlEvent::CleanRun)?;
        loop {
            // Snapshot the current overall state.
            let (state, last_control_event, exit) = {
                let guard = self.state.read().await;
                let exit = self.sched_rx.borrow();
                (
                    guard.overall_state(),
                    guard.last_control_event.clone(),
                    *exit,
                )
            };
            // Draw all remaining logs.
            loop {
                match self.log_rx.try_recv() {
                    Ok(record) => {
                        eprintln!("{}", record.format_for_print());
                    }
                    // Skip fallen-behind logs
                    Err(::tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => continue,
                    // Drained all logs
                    Err(::tokio::sync::broadcast::error::TryRecvError::Empty) => break,
                    // Channel unexpectedly closed
                    Err(e) => return Err(e.into()),
                }
            }
            // If a new error state is detected, abort the execution.
            if state == ExecutionState::Error && last_control_event != ControlEvent::Abort {
                log::error!("Aborting execution due to previous error.");
                self.ctrl_tx.send(ControlEvent::Abort)?;
                self.state
                    .write()
                    .await
                    .update_ui_state(ControlEvent::Abort)?;
            }
            if exit {
                // The main scheduler has exited, we can exit too.
                break;
            }
            ::tokio::time::sleep(::std::time::Duration::from_millis(10)).await;
        }

        Ok(())
    }

    fn handle_event(&mut self, event: Event) -> Option<Command> {
        match event {
            Event::Key(ke) if ke.code == KeyCode::Left => {
                self.progress_cursor = self.progress_cursor.saturating_sub(1)
            }
            Event::Key(ke) if ke.code == KeyCode::Right => {
                self.progress_cursor = self.progress_cursor.saturating_add(1);
            }
            Event::Key(ke) if ke.modifiers == KeyModifiers::ALT && ke.code == KeyCode::Up => {
                self.logs.scroll_up(1)
            }
            Event::Key(ke) if ke.modifiers == KeyModifiers::ALT && ke.code == KeyCode::Down => {
                self.logs.scroll_down(1);
            }
            Event::Key(ke) if ke.code == KeyCode::Up => self.logs.scroll_up(5),
            Event::Key(ke) if ke.code == KeyCode::Down => self.logs.scroll_down(5),
            Event::Key(ke) if ke.code == KeyCode::PageUp => self.logs.scroll_up(20),
            Event::Key(ke) if ke.code == KeyCode::PageDown => self.logs.scroll_down(20),
            Event::Key(ke) if ke.code == KeyCode::Esc => self.logs.reset_scroll(),
            Event::Key(key) => return self.prompt.on_key(key),
            Event::Mouse(me) if me.kind == MouseEventKind::ScrollUp => self.logs.scroll_up(5),
            Event::Mouse(me) if me.kind == MouseEventKind::ScrollDown => self.logs.scroll_down(5),
            _ => {}
        };
        None
    }

    async fn draw(&mut self, terminal: &mut Terminal<impl Backend>) -> Result<(), UiError> {
        let draw_snapshot = self.state.read().await.clone();
        let max_len = draw_snapshot
            .progress
            .keys()
            .map(|name| u16::try_from(name.len()).expect("Progress name too long"))
            .max()
            .expect("At least one job name exists")
            .clamp(3, 20);

        let height = terminal.size()?.height;
        let total_progress_bars = draw_snapshot.progress.len() as u16;
        let max_progress_bars = height
            .saturating_sub(height.saturating_div(2).max(9))
            .min(20);
        let num_progress_bars = total_progress_bars.min(max_progress_bars);

        // Clip the progress cursor if necessary
        if self.progress_cursor + num_progress_bars > total_progress_bars {
            self.progress_cursor = total_progress_bars.saturating_sub(num_progress_bars);
        }

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
                Constraint::Length(num_progress_bars + 1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Fill(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .areas(frame.area());
            frame.render_widget(
                Block::new().borders(Borders::TOP).title(
                    if num_progress_bars == total_progress_bars || num_progress_bars == 0 {
                        Line::from(vec![
                            Span::raw("│ "),
                            Span::raw("Progress").italic(),
                            Span::raw(" ├"),
                        ])
                        .left_aligned()
                    } else {
                        Line::from(vec![
                            Span::raw("│ "),
                            Span::raw("Progress").italic(),
                            Span::raw(format!(
                                " │ {}–{}/{total_progress_bars}",
                                self.progress_cursor + 1,
                                self.progress_cursor + num_progress_bars
                            )),
                            Span::raw(" ├"),
                        ])
                        .left_aligned()
                    },
                ),
                progress_head,
            );
            let constraints =
                std::iter::repeat_n(Constraint::Fill(1), (num_progress_bars + 1) as usize)
                    .collect::<Vec<_>>();
            let (progress_description, progress_bars) = Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints)
                .split(progress_area)
                .iter()
                .cloned()
                .split_first_owned()
                .expect("Progress area must have at least one bar");

            frame.render_widget(
                Line::from(if progress_description.width >= SEVENTY_SIX + max_len {
                    vec![
                        Span::raw(format!(
                            "{:width$}",
                            "",
                            width = (max_len.saturating_sub(3)) as usize
                        )),
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
                        Span::raw(format!(
                            "{:width$}",
                            "",
                            width = (max_len.saturating_sub(3)) as usize
                        )),
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
                .skip(self.progress_cursor as usize)
                .take(num_progress_bars as usize)
                .zip(progress_bars)
                .for_each(|((name, progress), bar)| {
                    draw_progress_gauge(frame, bar, name, progress, max_len);
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

            let logs_widget = self.logs.to_text(logs_area.width, logs_area.height);
            frame.render_widget(logs_widget, logs_area);

            let separator_widget = separator(self.logs.unread());
            frame.render_widget(separator_widget, logs_foot);

            let input_widget = self.prompt.to_line(draw_snapshot.overall_state().color());
            frame.render_widget(input_widget, input_area);
        })?;

        Ok(())
    }
}

fn separator(unread_logs: usize) -> Block<'static> {
    let title = if unread_logs == 0 {
        Line::from(vec![])
    } else {
        Line::from(vec![
            Span::raw("┤ "),
            Span::raw(format!("{unread_logs} unread, Esc to follow")).italic(),
            Span::raw(" │"),
        ])
        .right_aligned()
    };
    Block::new().borders(Borders::BOTTOM).title(title)
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

fn clamp_name(name: &str, max_len: u16) -> String {
    if name.len() > max_len as usize {
        format!("{}…", &name[..max_len as usize - 1])
    } else {
        format!("{name:>width$}", width = max_len as usize)
    }
}

/// Draws a progress gauge with a text label and a manual gauge.
fn draw_progress_gauge(
    frame: &mut Frame<'_>,
    area: Rect,
    name: &str,
    progress: &Progress,
    max_len: u16,
) {
    // Text area: "epsilon [ done/queue/ wait] "
    let text_length = 21 + max_len;
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
            Span::styled(
                clamp_name(name, max_len),
                Style::new().fg(progress.3.color()),
            ),
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
