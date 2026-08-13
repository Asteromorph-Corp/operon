use std::sync::OnceLock;

use crossterm::event::{Event, EventStream, KeyCode, KeyModifiers, MouseEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use futures::StreamExt;
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::logger::LogRecordReceiver;
use crate::scheduler::{ControlEvent, ControlEventSender, RunEventInner, SchedulerStateReceiver};
use crate::schema::{Progress, SharedProgressMap, TaskState};
use crate::ui::command::Command;
use crate::ui::command_prompt::CommandPrompt;
use crate::ui::log_view::LogView;
use crate::ui::{UiError, UiMode, UiOptions};
use crate::utils::SplitFirstOwned;

const MIN_TERMINAL_WIDTH_THRESHOLD: u16 = 27;
const VERBOSE_THRESHOLD: u16 = 76;
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
    run [OPTIONS]       Start a new run using the best available restoration
                        (unless specified by options).
                        --fresh, --rebuild, and --redo are mutually exclusive.
        -f, --fresh         Start a fresh run, ignoring any existing data.
        -r, --rebuild       Rebuild the run from trusted data before starting.
        -s, --skip <TASK>[ ...]
                            With --rebuild, do not rebuild the given 1 or more task(s).
        -R, --redo <TASK>[ ...]
                            Shorthand for --rebuild --skip <...>.
        -i, --redo-inconsistent-jobs
                            Rebuild the run even on a failed check,
                            ignoring tasks with corrupt data and their downstream tasks.
                            Cannot be used with --fresh.
                            Note that --redo <INCONSISTENT_TASKS> will NOT allow a rebuild
                            on a failed check without this flag.
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
    pause [OPTIONS] [<TASK>[ ...]]
                        Pause executing new jobs.
        -c, --cascade       Cascade the pause command to dependent tasks.
    resume [<TASK>[ ...]]
                        Resume paused tasks.
    help                Print this help message."#;
static MAX_TASK_NAME_LEN: OnceLock<u16> = OnceLock::new();

/// `Drop`-guarded terminal wrapper.
struct TerminalGuard<W: ::std::io::Write> {
    terminal: Terminal<CrosstermBackend<W>>,
}

impl<W: ::std::io::Write> ::std::ops::Deref for TerminalGuard<W> {
    type Target = Terminal<CrosstermBackend<W>>;

    fn deref(&self) -> &Self::Target {
        &self.terminal
    }
}

impl<W: ::std::io::Write> ::std::ops::DerefMut for TerminalGuard<W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.terminal
    }
}

impl<W: ::std::io::Write> Drop for TerminalGuard<W> {
    fn drop(&mut self) {
        if let Err(err) = disable_raw_mode() {
            tracing::error!("Failed to disable raw mode while restoring the terminal: {err}");
        }
        if let Err(err) = execute!(self.terminal.backend_mut(), LeaveAlternateScreen) {
            tracing::error!(
                "Failed to leave the alternate screen while restoring the terminal: {err}"
            );
        }
        if let Err(err) = self.terminal.show_cursor() {
            tracing::error!("Failed to show the cursor while restoring the terminal: {err}");
        }
    }
}

/// The main UI loop that handles user input and updates the UI state.
pub struct UiLoop {
    mode: UiMode,
    progresses: SharedProgressMap,
    progress_cursor: u16,
    logs: LogView,
    prompt: CommandPrompt,
    log_rx: LogRecordReceiver,
    ctrl_tx: ControlEventSender,
    sched_rx: SchedulerStateReceiver,
    finished: bool,
    exit_on_finish: bool,
}

impl UiLoop {
    /// Create a new UI loop with the given state, primary upper bound, log receiver,
    /// control event sender, and recovery state receiver.
    pub fn new(
        progresses: SharedProgressMap,
        log_rx: LogRecordReceiver,
        ctrl_tx: ControlEventSender,
        sched_rx: SchedulerStateReceiver,
        options: UiOptions,
    ) -> Self {
        Self {
            mode: options.mode,
            progresses,
            progress_cursor: 0u16,
            logs: LogView::new(options.log_buffer_size),
            prompt: CommandPrompt::default(),
            log_rx,
            ctrl_tx,
            sched_rx,
            finished: false,
            exit_on_finish: false,
        }
    }

    pub fn is_task(&self, task_name: &str) -> bool {
        self.progresses.0.contains_key(task_name)
    }

    /// Idempotently records the scheduler as gone and paints every task as errored.
    async fn mark_scheduler_lost(&mut self) {
        if self.finished {
            return;
        }
        tracing::error!(
            "Operon's UI lost contact with the scheduler and cannot continue execution."
        );
        self.finished = true;
        for progress in self.progresses.0.iter() {
            progress.1.write().await.set_state(TaskState::Error);
        }
    }

    /// Sends a control event, treating a dropped scheduler channel as a graceful fatal exit.
    async fn send_control(&mut self, event: ControlEvent) {
        if self.ctrl_tx.send(event).await.is_err() {
            self.mark_scheduler_lost().await;
        }
    }

    pub async fn run(self) -> Result<(), UiError> {
        match self.mode {
            UiMode::Interactive => self.run_interactive().await,
            UiMode::Headless => self.run_headless().await,
        }
    }

    pub async fn run_interactive(mut self) -> Result<(), UiError> {
        // Capture stdout/stderr and forward to tracing (must be after subscriber setup).
        // Best-effort: if capture fails or is unavailable (non-Unix), fall back to normal stdout.
        #[cfg(unix)]
        let (_fd_redirect, original_stdout) = match crate::ui::capture_std_outputs() {
            Ok(redirect) => {
                let stdout = redirect.original_fd(&std::io::stdout());
                (Some(redirect), stdout)
            }
            Err(_) => {
                tracing::warn!(
                    "Failed to capture stdout/stderr, falling back to normal output. This may cause display issues in the TUI."
                );
                (None, None)
            }
        };

        #[cfg(not(unix))]
        let original_stdout: Option<std::fs::File> = None;

        enable_raw_mode()?;

        let mut stdout_writer: Box<dyn ::std::io::Write> = match original_stdout {
            Some(file) => Box::new(file),
            None => Box::new(::std::io::stdout()),
        };

        execute!(stdout_writer, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout_writer);
        let mut terminal = TerminalGuard {
            terminal: Terminal::new(backend)?,
        };
        terminal.clear()?;

        let snapshot = self.progresses.snapshot().await;
        MAX_TASK_NAME_LEN
            .set(
                snapshot
                    .0
                    .keys()
                    .map(|name| u16::try_from(name.len()).expect("Progress name too long"))
                    .max()
                    .expect("At least one task name exists")
                    .clamp(4, 20),
            )
            .ok();

        // Main loop for the UI.
        let mut events = EventStream::new();
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(50));
        loop {
            if self.finished && self.exit_on_finish {
                break;
            }

            tokio::select! {
                scheduler_exit = &mut self.sched_rx, if !self.finished => {
                    match scheduler_exit {
                        // Scheduler exited gracefully with a signal to exit the UI
                        Ok(true) => break,
                        // Scheduler exited gracefully with a signal to continue the UI
                        Ok(false) => self.finished = true,
                        // Scheduler dropped the channel and early-returned
                        Err(_) => self.mark_scheduler_lost().await,
                    }
                }

                evt = events.next() => {
                    let Some(evt) = evt else {
                        // Stream closed, exit the UI.
                        break;
                    };

                    if let Some(command) = self.handle_event(evt?) {
                        let exit_ui = self.execute_command(command).await?;

                        if exit_ui {
                            break;
                        }
                    };
                }
                Ok(record) = self.log_rx.recv() => {
                    let width = terminal.size()?.width;
                    let verbose = width
                        >= VERBOSE_THRESHOLD
                            + MAX_TASK_NAME_LEN
                                .get()
                                .ok_or(UiError::Other("Max task name length not set".to_string()))?;
                    self.logs.push(record, width, verbose);
                }
                _ = interval.tick() => self.draw(&mut terminal).await?,
            }
        }

        Ok(())
    }

    pub async fn run_headless(mut self) -> Result<(), UiError> {
        // Main loop for the headless UI.
        // Recovery is disabled for this mode,
        // so we always run fresh off the bat and wait
        // until everything finishes or something errors.
        self.send_control(ControlEvent::FRESH_RUN).await;
        loop {
            if self.finished {
                break;
            }

            tokio::select! {
                scheduler_exit = &mut self.sched_rx, if !self.finished => {
                    match scheduler_exit {
                        Ok(true) => break,
                        Ok(false) => self.finished = true,
                        Err(_) => self.mark_scheduler_lost().await,
                    }
                }
                Ok(record) = self.log_rx.recv() => record.write_to_posix(&mut std::io::stdout(), &mut std::io::stderr())?,
            }

            // Snapshot the current overall state.
            let state = self.progresses.snapshot().await.overall_state();

            // If a new error state is detected, abort the execution.
            if state == TaskState::Error {
                tracing::error!("Aborting execution due to previous error.");
                self.send_control(ControlEvent::FORCE_QUIT).await;
            }
        }

        // Drain records buffered past the scheduler's exit so a final message,
        // such as the wall-clock summary, still reaches the terminal.
        loop {
            match self.log_rx.try_recv() {
                Ok(record) => {
                    record.write_to_posix(&mut std::io::stdout(), &mut std::io::stderr())?
                }
                Err(::tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => continue,
                Err(_) => break,
            }
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

    async fn execute_command(&mut self, command: Command) -> Result<bool, UiError> {
        if self.finished {
            match command {
                Command::Run { .. } => tracing::warn!(
                    "Already run. Use `exit` or `quit` to terminate the current session before starting a new run."
                ),
                Command::Check { .. } => {
                    tracing::warn!("Cannot check after the run has already started.")
                }
                Command::Quit { no_exit: true, .. } => tracing::warn!("Nothing to quit."),
                Command::Quit { .. } | Command::Exit => return Ok(true),
                Command::Pause { .. } => tracing::warn!("Nothing to pause."),
                Command::Resume { .. } => tracing::warn!("Nothing to resume."),
                Command::Clear => {
                    self.logs.clear();
                    self.log_rx = self.log_rx.resubscribe();
                }
                Command::Help => tracing::info!("{HELP_TEXT}"),
            };

            return Ok(false);
        }

        match command {
            Command::Run {
                fresh,
                rebuild,
                skip,
                redo,
                redo_inconsistent_tasks,
            } => {
                let event_inner = if fresh {
                    RunEventInner::Fresh
                } else if rebuild {
                    RunEventInner::Rebuild {
                        skip: skip.into_iter().collect(),
                        redo_inconsistent_tasks,
                    }
                } else if !redo.is_empty() {
                    RunEventInner::Rebuild {
                        skip: redo.into_iter().collect(),
                        redo_inconsistent_tasks,
                    }
                } else {
                    RunEventInner::Unspecified {
                        redo_inconsistent_tasks,
                    }
                };
                if let RunEventInner::Rebuild { skip, .. } = &event_inner
                    && let Some(invalid_task) = skip.iter().find(|task| !self.is_task(task))
                {
                    tracing::error!("Unknown task name: {invalid_task}")
                } else {
                    self.send_control(ControlEvent::Run(event_inner)).await
                }
            }
            Command::Check { mode } => self.send_control(ControlEvent::Check { mode }).await,
            Command::Quit { force, no_exit } => {
                self.send_control(ControlEvent::Quit { force, no_exit })
                    .await;
                self.exit_on_finish = !no_exit;
            }
            Command::Exit => self.send_control(ControlEvent::Exit).await,
            Command::Pause { targets, cascade } => {
                if let Some(invalid_task) = targets.iter().find(|task| !self.is_task(task)) {
                    tracing::error!("Unknown task name: {invalid_task}")
                } else {
                    self.send_control(ControlEvent::Pause { targets, cascade })
                        .await
                }
            }
            Command::Resume { targets } => {
                if let Some(invalid_task) = targets.iter().find(|task| !self.is_task(task)) {
                    tracing::error!("Unknown task name: {invalid_task}")
                } else {
                    self.send_control(ControlEvent::Resume { targets }).await
                }
            }
            Command::Clear => {
                self.logs.clear();
                self.log_rx = self.log_rx.resubscribe();
            }
            Command::Help => tracing::info!("{HELP_TEXT}"),
        }

        Ok(false)
    }

    async fn draw(&mut self, terminal: &mut Terminal<impl Backend>) -> Result<(), UiError> {
        let size = terminal.size()?;
        let max_len = *MAX_TASK_NAME_LEN
            .get()
            .ok_or(UiError::Other("Max task name length not set".to_string()))?;

        let min_width = MIN_TERMINAL_WIDTH_THRESHOLD + max_len;
        if size.width < min_width || size.height < 10 {
            terminal.draw(|frame| {
                let vertical = Layout::vertical([
                    Constraint::Fill(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Fill(1),
                ])
                .split(frame.area());
                let dark_gray = ::ratatui::style::Style::new().dark_gray();
                let red = ::ratatui::style::Style::new().red();
                frame.render_widget(
                    Line::from("Terminal size too small.".to_string())
                        .dark_gray()
                        .centered(),
                    vertical[1],
                );
                frame.render_widget(
                    Line::from(format!("Need: {min_width}x10."))
                        .dark_gray()
                        .centered(),
                    vertical[2],
                );
                frame.render_widget(
                    Line::from(vec![
                        Span::styled("Current: ", dark_gray),
                        Span::styled(
                            format!("{}", size.width),
                            if size.width < min_width {
                                red
                            } else {
                                dark_gray
                            },
                        ),
                        Span::styled("x", dark_gray),
                        Span::styled(
                            format!("{}", size.height),
                            if size.height < 10 { red } else { dark_gray },
                        ),
                    ])
                    .centered(),
                    vertical[3],
                );
            })?;

            return Ok(());
        }

        let draw_snapshot = self.progresses.snapshot().await;
        let verbose = size.width >= VERBOSE_THRESHOLD + max_len;

        let height = size.height;
        let total_progress_bars = draw_snapshot.0.len() as u16;
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
                Line::from(if verbose {
                    vec![
                        Span::raw(format!(
                            "{:width$}",
                            "",
                            width = (max_len.saturating_sub(4)) as usize
                        )),
                        Span::raw("task").underlined(),
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
                            width = (max_len.saturating_sub(4)) as usize
                        )),
                        Span::raw("task").underlined(),
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
                .0
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

            let logs_widget = self.logs.format(logs_area.width, logs_area.height, verbose);
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
    frame.render_widget(
        Line::from(vec![
            Span::styled(
                clamp_name(name, max_len),
                Style::new().fg(progress.state.color()),
            ),
            Span::raw(format!(
                " [{}/{}/{}] ",
                five_format(progress.done),
                five_format(progress.queued),
                five_format(progress.waiting)
            )),
        ]),
        text_area,
    );

    // Gauge area: manual gauge with Span, surrounded by borders
    let gauge_length = gauge_area.width;
    let total = progress.done + progress.queued + progress.waiting;

    let done_length = if total > 0 {
        ((progress.done as f64 / total as f64) * gauge_length as f64).round() as u16
    } else {
        0
    };
    let queued_length = if total > 0 {
        ((progress.queued as f64 / total as f64) * gauge_length as f64).round() as u16
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
        Style::default().fg(progress.state.color()),
    );
    let queued_span = Span::styled(
        "░".repeat(queued_length as usize),
        Style::default().fg(progress.state.color()),
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
