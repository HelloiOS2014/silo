use anyhow::{Context, Result};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    process::{Command, ExitStatus},
};

use crate::{env_path, path_policy::validate_cwd, runtime_env::build_child_env};

pub fn run(env: &str, cwd: Option<PathBuf>, command: Vec<String>) -> Result<ExitStatus> {
    let (manifest, env_root) = env_path::load_manifest(env)?;
    let secrets = env_path::resolve_secrets(&manifest, &env_root)?;

    let cwd = match cwd {
        Some(c) => c,
        None => {
            if manifest.inherit_cwd {
                std::env::current_dir()?
            } else {
                manifest.root.join("home")
            }
        }
    };
    let (cwd, _shared) = validate_cwd(&cwd, &manifest.shared_paths)?;

    let pid = std::process::id();
    let run_dir = env_root.join("run").join(pid.to_string());
    fs::create_dir_all(&run_dir)?;
    let run_dir_str = run_dir.to_string_lossy().to_string();

    let host: BTreeMap<String, String> = std::env::vars().collect();
    let child_env = build_child_env(&manifest, &host, secrets, Some(&run_dir_str));

    let (program, args) = command
        .split_first()
        .context("command cannot be empty")?;

    let status = Command::new(program)
        .args(args)
        .current_dir(&cwd)
        .env_clear()
        .envs(child_env)
        .status()?;

    let _ = fs::remove_dir_all(&run_dir);

    Ok(status)
}
