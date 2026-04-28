use std::str::FromStr;

use clap::{Parser, Subcommand};

use crate::schema::CheckMode;

#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
#[clap(rename_all = "kebab-case")]
pub enum Command {
    /// Start a new run using the best available restoration.
    Run {
        /// Start a fresh run, ignoring any existing data.
        #[clap(short, long, conflicts_with_all = ["rebuild", "skip", "redo"])]
        fresh: bool,
        /// Rebuild the run from trusted data before starting.
        #[clap(short, long, conflicts_with_all = ["fresh", "redo"])]
        rebuild: bool,
        /// Do not rebuild the given jobs when `--rebuild` is specified.
        #[clap(short, long, requires = "rebuild", num_args = 1..)]
        skip: Vec<String>,
        /// Shorthand for `--rebuild --skip <...>`.
        #[clap(short = 'R', long, conflicts_with_all = ["fresh", "rebuild", "skip"], num_args = 1..)]
        redo: Vec<String>,
        /// Rebuild the run while skipping inconsistent jobs.
        #[clap(short = 'i', long, conflicts_with = "fresh")]
        redo_inconsistent_jobs: bool,
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

/// Helper struct to parse commands from user input.
#[derive(Parser)]
#[clap(name = "operon")]
#[command(disable_help_flag = true, disable_help_subcommand = true)]
struct PromptParser {
    #[clap(subcommand)]
    pub action: Command,
}

impl FromStr for Command {
    type Err = clap::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let args = std::iter::once("operon").chain(s.split_whitespace());
        let helper = PromptParser::try_parse_from(args)?;
        Ok(helper.action)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::run_fresh("run --fresh", Command::Run { fresh: true, rebuild: false, skip: vec![], redo: vec![], redo_inconsistent_jobs: false })]
    #[case::run_rebuild("run --rebuild", Command::Run { fresh: false, rebuild: true, skip: vec![], redo: vec![], redo_inconsistent_jobs: false })]
    #[case::run_skip("run --rebuild --skip job1 job2", Command::Run { fresh: false, rebuild: true, skip: vec!["job1".to_owned(), "job2".to_owned()], redo: vec![], redo_inconsistent_jobs: false })]
    #[case::run_redo("run --redo job1 job2", Command::Run { fresh: false, rebuild: false, skip: vec![], redo: vec!["job1".to_owned(), "job2".to_owned()], redo_inconsistent_jobs: false })]
    #[case::run_redo_inconsistent("run --redo-inconsistent-jobs", Command::Run { fresh: false, rebuild: false, skip: vec![], redo: vec![], redo_inconsistent_jobs: true })]
    #[case::check_trust_all("check --mode trust-all", Command::Check { mode: CheckMode::TrustAll })]
    #[case::check_metadata_only("check --mode metadata-only", Command::Check { mode: CheckMode::MetadataOnly })]
    #[case::check_quick("check", Command::Check { mode: CheckMode::Quick })]
    #[case::check_exhaustive("check --mode exhaustive", Command::Check { mode: CheckMode::Exhaustive })]
    #[case::quit_force("quit --force", Command::Quit { force: true, no_exit: false })]
    #[case::quit_no_exit("quit --no-exit", Command::Quit { force: false, no_exit: true })]
    #[case::pause_all("pause", Command::Pause { targets: vec![], cascade: false })]
    #[case::pause_specific("pause job1 job2 --cascade", Command::Pause { targets: vec!["job1".to_owned(), "job2".to_owned()], cascade: true })]
    #[case::resume_all("resume", Command::Resume { targets: vec![] })]
    #[case::resume_specific("resume job1 job2", Command::Resume { targets: vec!["job1".to_owned(), "job2".to_owned()] })]
    #[case::exit("exit", Command::Exit)]
    #[case::clear("clear", Command::Clear)]
    #[case::help("help", Command::Help)]
    fn test_parse_command(#[case] input: &str, #[case] expected: Command) {
        let action = Command::from_str(input).unwrap();
        assert_eq!(action, expected);
    }
}
