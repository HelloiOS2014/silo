use anyhow::Result;
use std::{fs, path::PathBuf};

pub fn run(env: &str) -> Result<()> {
    let root = env_root(env)?;
    let was_present = root.exists();

    for suffix in ["home", "config", "cache", "tmp", "run"] {
        fs::create_dir_all(root.join(suffix))?;
    }

    let manifest_path = root.join("manifest.toml");
    if !manifest_path.exists() {
        fs::write(&manifest_path, default_manifest(env, &root))?;
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
