use aienv::{
    cli::{Cli, Commands},
    commands,
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
