use anyhow::{Result, bail};
use std::{collections::BTreeMap, fs, process::Command};

use crate::{env_path, runtime_env::build_child_env};

pub fn run(env: &str, force: bool) -> Result<()> {
    let (manifest, env_root) = env_path::load_manifest(env)?;
    let secrets = env_path::resolve_secrets(&manifest, &env_root)?;

    let marker = env_root.join(".setup-done");

    if marker.exists() && !force {
        println!("setup already completed (use --force to re-run)");
        return Ok(());
    }

    if manifest.setup.on_init.is_empty() {
        println!("no setup hooks defined");
        return Ok(());
    }

    let host: BTreeMap<String, String> = std::env::vars().collect();
    let child_env = build_child_env(&manifest, &host, secrets, None);

    let env_home = manifest.root.join("home");

    for (i, cmd) in manifest.setup.on_init.iter().enumerate() {
        println!("[setup {}/{}] {cmd}", i + 1, manifest.setup.on_init.len());

        let status = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .env_clear()
            .envs(&child_env)
            .current_dir(&env_home)
            .status()?;

        if !status.success() {
            bail!(
                "setup command failed (exit {}): {cmd}",
                status.code().unwrap_or(-1)
            );
        }
    }

    fs::write(&marker, "")?;
    println!("setup complete ({} commands)", manifest.setup.on_init.len());

    Ok(())
}
