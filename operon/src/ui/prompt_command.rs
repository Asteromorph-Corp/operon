use clap::{Parser, Subcommand};

#[derive(Parser, Debug, Clone)]
#[clap(name = "operon")]
#[command(disable_help_flag = true, disable_help_subcommand = true)]
pub struct PromptCommand {
    #[clap(subcommand)]
    pub action: Action,
}

#[derive(Subcommand, Debug, Clone)]
#[clap(rename_all = "kebab-case")]
pub enum Action {
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

    /// Exit the UI.
    Exit,

    /// Clear the log buffer.
    Clear,

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

    /// Print help.
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
