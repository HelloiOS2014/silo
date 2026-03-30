mod commands;

use aienv::{
    cli::{Cli, Commands},
};
use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { env } => commands::init::run(&env)?,
        _ => todo!("other commands not implemented yet"),
    }

    Ok(())
}
