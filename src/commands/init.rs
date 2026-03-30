use crate::manifest::Manifest;
use anyhow::{anyhow, Result};
use std::{fs, path::PathBuf};

pub fn run(env: &str) -> Result<()> {
    validate_env_name(env)?;
    let root = env_root(env)?;
    let was_present = root.exists();

    for suffix in ["home", "config", "cache", "tmp", "run"] {
        fs::create_dir_all(root.join(suffix))?;
    }

    let manifest_path = root.join("manifest.toml");
    if !manifest_path.exists() {
        let manifest = default_manifest(env, &root);
        // Keep init-generated manifests parseable by the current runtime.
        Manifest::parse(&manifest).map_err(|err| anyhow!(err.to_string()))?;
        fs::write(&manifest_path, manifest)?;
    }

    let init_path = root.join("env.zsh");
    if !init_path.exists() {
        fs::write(&init_path, format!("export AI_ENV={env}\n"))?;
    }

    if was_present {
        println!("environment {env} already initialized");
    } else {
        println!("initialized environment {env}");
    }

    Ok(())
}

fn validate_env_name(env: &str) -> Result<()> {
    if env.is_empty() {
        return Err(anyhow!("environment name cannot be empty"));
    }

    if env.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_') {
        Ok(())
    } else {
        Err(anyhow!(
            "environment name may only contain ASCII letters, digits, '-' and '_'"
        ))
    }
}

fn env_root(env: &str) -> Result<PathBuf> {
    let home = std::env::var("HOME")?;
    Ok(PathBuf::from(home).join(".aienv").join(env))
}

fn default_manifest(env: &str, root: &PathBuf) -> String {
    format!(
        "id = \"{env}\"\nroot = \"{}\"\ninherit_cwd = true\nshared_paths = []\n\n[env]\nallow = [\"TERM\", \"LANG\", \"LC_ALL\", \"COLORTERM\", \"PATH\"]\ndeny = [\"OPENAI_API_KEY\", \"ANTHROPIC_API_KEY\", \"GEMINI_API_KEY\", \"SSH_AUTH_SOCK\"]\n\n[env.set]\nAI_ENV = \"{env}\"\n\n[secrets]\nprovider = \"keychain\"\nitems = []\n\n[shell]\nprogram = \"/bin/zsh\"\ninit = \"env.zsh\"\n\n[network]\nmode = \"default\"\n",
        root.display()
    )
}
