use clap::{Parser, Subcommand};

#[derive(Parser, Debug, Clone)]
#[clap(name = "operon")]
#[command(disable_help_flag = true, disable_help_subcommand = true)]
pub struct PromptCommand {
    #[clap(subcommand)]
    pub action: Action,
}

#[derive(Subcommand, Debug, Clone)]
#[clap(rename_all = "snake_case")]
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
    Check,

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
