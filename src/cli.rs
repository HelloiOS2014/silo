use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "aienv",
    version,
    about = "Run commands inside isolated AI identity environments"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Initialize a new environment
    Init {
        #[arg(short = 'e', long, help = "Environment name")]
        env: String,
    },
    /// Execute a command in an environment
    Exec {
        #[arg(short = 'e', long, help = "Environment name")]
        env: String,
        #[arg(long, help = "Allocate a TTY for interactive commands")]
        tty: bool,
        #[arg(long, help = "Override working directory")]
        cwd: Option<PathBuf>,
        #[arg(last = true, required = true, help = "Command to execute")]
        command: Vec<String>,
    },
    /// Enter an environment shell
    Shell {
        #[arg(short = 'e', long, help = "Environment name")]
        env: String,
        #[arg(long, help = "Override working directory")]
        cwd: Option<PathBuf>,
    },
    /// List all environments
    Ls,
    /// Show resolved environment configuration
    Show {
        #[arg(short = 'e', long, help = "Environment name")]
        env: String,
    },
}
