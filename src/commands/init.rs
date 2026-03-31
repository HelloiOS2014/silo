use crate::{env_path, manifest::Manifest};
use anyhow::{Result, anyhow};
use std::fs;

pub fn run(env: &str) -> Result<()> {
    validate_env_name(env)?;
    let root = env_path::env_root(env)?;
    let was_present = root.exists();

    for suffix in ["home", "config", "cache", "data", "state", "tmp", "run"] {
        fs::create_dir_all(root.join(suffix))?;
    }

    let manifest_path = root.join("manifest.toml");
    if !manifest_path.exists() {
        let manifest = default_manifest(env, &root);
        Manifest::parse(&manifest).map_err(|err| anyhow!(err.to_string()))?;
        fs::write(&manifest_path, manifest)?;
    }

    let init_path = root.join("env.zsh");
    if !init_path.exists() {
        fs::write(&init_path, format!("export AI_ENV={env}\n"))?;
    }

    let secrets_path = root.join("secrets.env");
    if !secrets_path.exists() {
        fs::write(&secrets_path, "")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&secrets_path, fs::Permissions::from_mode(0o600))?;
        }
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

    if env
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        Ok(())
    } else {
        Err(anyhow!(
            "environment name may only contain ASCII letters, digits, '-' and '_'"
        ))
    }
}

fn default_manifest(env: &str, root: &std::path::Path) -> String {
    format!(
        r#"id = "{env}"
root = "{root}"
inherit_cwd = true
shared_paths = []

[env]
allow = ["TERM", "LANG", "LC_ALL", "COLORTERM", "PATH"]
deny = [
  "SSH_AUTH_SOCK",
  "OPENAI_API_KEY",
  "ANTHROPIC_API_KEY",
  "GEMINI_API_KEY",
  "AWS_ACCESS_KEY_ID",
  "AWS_SECRET_ACCESS_KEY",
  "AWS_SESSION_TOKEN",
  "GOOGLE_APPLICATION_CREDENTIALS",
  "AZURE_CLIENT_ID",
  "AZURE_CLIENT_SECRET",
  "AZURE_TENANT_ID",
  "http_proxy",
  "https_proxy",
  "ALL_PROXY",
]

[env.set]
AI_ENV = "{env}"

[secrets]
provider = "keychain"
items = []

[shell]
program = "/bin/zsh"
init = "env.zsh"

[network]
mode = "default"
"#,
        root = root.display()
    )
}
