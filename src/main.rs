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
        Commands::Exec {
            env,
            tty: false,
            cwd,
            command,
        } => {
            let status = commands::exec::run(&env, cwd, command)?;
            std::process::exit(status.code().unwrap_or(1));
        }
        Commands::Exec { tty: true, .. } => todo!("tty path comes next"),
        Commands::Shell { .. } => todo!("shell path comes next"),
        Commands::Ls => todo!("ls comes later"),
        Commands::Show { .. } => todo!("show comes later"),
    }

    Ok(())
}
