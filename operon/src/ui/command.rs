use std::str::FromStr;

use clap::{Parser, Subcommand};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Scheduler(SchedulerCommand),
    Ui(UiCommand),
}

impl Command {
    pub const EXIT: Self = Command::Ui(UiCommand::Exit);
    pub const CLEAR: Self = Command::Ui(UiCommand::Clear);
    pub const HELP: Self = Command::Ui(UiCommand::Help);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulerCommand {
    Run { fresh: bool, rebuild: bool },
    Check { mode: CheckMode },
    Quit { force: bool, no_exit: bool },
    Pause { targets: Vec<String>, cascade: bool },
    Resume { targets: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiCommand {
    Exit,
    Clear,
    Help,
}

#[derive(clap::ValueEnum, Debug, Clone, PartialEq, Eq, Copy)]
#[clap(rename_all = "kebab-case")]
pub enum CheckMode {
    TrustAll,
    MetadataOnly,
    Quick,
    Exhaustive,
}

/// Helper struct to parse commands from user input.
#[derive(Parser)]
#[clap(name = "operon")]
#[command(disable_help_flag = true, disable_help_subcommand = true)]
struct PromptParser {
    #[clap(subcommand)]
    pub action: PromptCommand,
}

#[derive(Subcommand)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
#[clap(rename_all = "kebab-case")]
enum PromptCommand {
    /// Start a new run using the best available restoration.
    Run {
        /// Start a fresh run, ignoring any existing data.
        #[clap(short, long)]
        fresh: bool,
        /// Rebuild the run from trusted data before starting.
        #[clap(short, long)]
        rebuild: bool,
    },

    /// Check the consistency of the data from the last run.
    Check {
        /// Mode of the consistency check. Defaults to "quick".
        ///
        /// - trust-all: Assume all data is trustworthy, skipping checks.
        /// - metadata-only: Check only metadata consistency.
        /// - quick: Perform a metadata check plus data validation only at upper boundaries.
        /// - exhaustive: Perform a full consistency check of all data. (Can be very slow.)
        #[clap(short, long, default_value = "quick")]
        mode: CheckMode,
    },

    /// Quit Operon.
    Quit {
        /// Force quit, ignoring running jobs.
        #[clap(short, long)]
        force: bool,
        /// Don't exit the UI.
        #[clap(short, long = "no-exit")]
        no_exit: bool,
    },

    /// Pause jobs.
    Pause {
        /// Target job(s) to pause.
        /// If not specified, pauses all jobs.
        #[clap()]
        targets: Vec<String>,
        /// Whether to cascade down the pause command
        /// to all dependent jobs.
        #[clap(short, long)]
        cascade: bool,
    },

    /// Resume jobs.
    Resume {
        /// Target job(s) to resume.
        /// If not specified, resumes all jobs.
        #[clap()]
        targets: Vec<String>,
    },

    /// Exit the UI.
    Exit,

    /// Clear the log buffer.
    Clear,

    /// Print help.
    Help,
}

impl FromStr for PromptCommand {
    type Err = clap::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let args = std::iter::once("operon").chain(s.split_whitespace());
        let helper = PromptParser::try_parse_from(args)?;
        Ok(helper.action)
    }
}

impl FromStr for Command {
    type Err = clap::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let helper = PromptCommand::from_str(s)?;
        let command = match helper {
            PromptCommand::Run { fresh, rebuild } => {
                Command::Scheduler(SchedulerCommand::Run { fresh, rebuild })
            }
            PromptCommand::Check { mode } => Command::Scheduler(SchedulerCommand::Check { mode }),
            PromptCommand::Quit { force, no_exit } => {
                Command::Scheduler(SchedulerCommand::Quit { force, no_exit })
            }
            PromptCommand::Pause { targets, cascade } => {
                Command::Scheduler(SchedulerCommand::Pause { targets, cascade })
            }
            PromptCommand::Resume { targets } => {
                Command::Scheduler(SchedulerCommand::Resume { targets })
            }
            PromptCommand::Exit => Command::Ui(UiCommand::Exit),
            PromptCommand::Clear => Command::Ui(UiCommand::Clear),
            PromptCommand::Help => Command::Ui(UiCommand::Help),
        };
        Ok(command)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::run_fresh("run --fresh", PromptCommand::Run { fresh: true, rebuild: false })]
    #[case::run_rebuild("run --rebuild", PromptCommand::Run { fresh: false, rebuild: true })]
    #[case::check_trust_all("check --mode trust-all", PromptCommand::Check { mode: CheckMode::TrustAll })]
    #[case::check_metadata_only("check --mode metadata-only", PromptCommand::Check { mode: CheckMode::MetadataOnly })]
    #[case::check_quick("check", PromptCommand::Check { mode: CheckMode::Quick })]
    #[case::check_exhaustive("check --mode exhaustive", PromptCommand::Check { mode: CheckMode::Exhaustive })]
    #[case::quit_force("quit --force", PromptCommand::Quit { force: true, no_exit: false })]
    #[case::quit_no_exit("quit --no-exit", PromptCommand::Quit { force: false, no_exit: true })]
    #[case::pause_all("pause", PromptCommand::Pause { targets: vec![], cascade: false })]
    #[case::pause_specific("pause job1 job2 --cascade", PromptCommand::Pause { targets: vec!["job1".to_owned(), "job2".to_owned()], cascade: true })]
    #[case::resume_all("resume", PromptCommand::Resume { targets: vec![] })]
    #[case::resume_specific("resume job1 job2", PromptCommand::Resume { targets: vec!["job1".to_owned(), "job2".to_owned()] })]
    #[case::exit("exit", PromptCommand::Exit)]
    #[case::clear("clear", PromptCommand::Clear)]
    #[case::help("help", PromptCommand::Help)]
    fn test_parse_command(#[case] input: &str, #[case] expected: PromptCommand) {
        let action = PromptCommand::from_str(input).unwrap();
        assert_eq!(action, expected);
    }
}
