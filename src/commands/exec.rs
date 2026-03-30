use anyhow::{Context, Result};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    process::{Command, ExitStatus},
};

use crate::{manifest::Manifest, path_policy::validate_cwd, runtime_env::build_child_env};

pub fn run(env: &str, cwd: Option<PathBuf>, command: Vec<String>) -> Result<ExitStatus> {
    let manifest = load_manifest(env)?;
    let cwd = cwd.unwrap_or(std::env::current_dir()?);
    let cwd = validate_cwd(&cwd, &manifest.shared_paths)?;

    let host: BTreeMap<String, String> = std::env::vars().collect();
    let child_env = build_child_env(&manifest, &host, BTreeMap::new());

    let (program, args) = command
        .split_first()
        .context("command cannot be empty")?;

    let status = Command::new(program)
        .args(args)
        .current_dir(&cwd)
        .env_clear()
        .envs(child_env)
        .status()?;

    Ok(status)
}

pub(crate) fn load_manifest(env: &str) -> Result<Manifest> {
    let home = std::env::var("HOME")?;
    let path = PathBuf::from(home).join(".aienv").join(env).join("manifest.toml");
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read manifest {}", path.display()))?;
    Ok(Manifest::parse(&raw)?)
}
