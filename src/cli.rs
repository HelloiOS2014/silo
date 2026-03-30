use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "aienv")]
#[command(about = "Run commands inside isolated AI identity environments")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Init {
        #[arg(long)]
        env: String,
    },
    Exec {
        #[arg(long)]
        env: String,
        #[arg(long)]
        tty: bool,
        #[arg(long)]
        cwd: Option<PathBuf>,
        #[arg(last = true, required = true)]
        command: Vec<String>,
    },
    Shell {
        #[arg(long)]
        env: String,
        #[arg(long)]
        cwd: Option<PathBuf>,
    },
    Ls,
    Show {
        #[arg(long)]
        env: String,
    },
}
