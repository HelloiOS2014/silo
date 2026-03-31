use anyhow::Result;
use clap::Parser;
use silo::{
    cli::{Cli, Commands},
    commands,
};
use std::process::ExitStatus;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { env } => commands::init::run(&env)?,
        Commands::Exec {
            env,
            tty: _,
            cwd,
            command,
        } => {
            let status = commands::exec::run(&env, cwd, command)?;
            std::process::exit(exit_code(&status));
        }
        Commands::Shell { env, cwd } => {
            let code = commands::shell::run(&env, cwd)?;
            std::process::exit(code);
        }
        Commands::Setup { env, force } => commands::setup::run(&env, force)?,
        Commands::Ls => commands::ls::run()?,
        Commands::Show { env } => commands::show::run(&env)?,
    }

    Ok(())
}

fn exit_code(status: &ExitStatus) -> i32 {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        status
            .code()
            .unwrap_or_else(|| 128 + status.signal().unwrap_or(1))
    }
    #[cfg(not(unix))]
    {
        status.code().unwrap_or(1)
    }
}
