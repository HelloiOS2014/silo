use crate::manifest::Manifest;
use crate::secrets;
use anyhow::{Context, Result, bail};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

pub fn aienv_root() -> Result<PathBuf> {
    if let Ok(root) = std::env::var("AIENV_ROOT") {
        return Ok(PathBuf::from(root));
    }
    let home = std::env::var("HOME").context("HOME not set")?;
    Ok(PathBuf::from(home).join(".aienv"))
}

pub fn env_root(env: &str) -> Result<PathBuf> {
    Ok(aienv_root()?.join(env))
}

pub fn load_manifest(env: &str) -> Result<(Manifest, PathBuf)> {
    let root = env_root(env)?;
    let path = root.join("manifest.toml");
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read manifest {}", path.display()))?;
    let manifest = Manifest::parse(&raw)?;

    if manifest.id != env {
        bail!(
            "manifest id \"{}\" does not match environment name \"{}\"",
            manifest.id,
            env
        );
    }

    if manifest.root != root {
        bail!(
            "manifest root \"{}\" does not match environment directory \"{}\"",
            manifest.root.display(),
            root.display()
        );
    }

    Ok((manifest, root))
}

pub fn resolve_secrets(manifest: &Manifest, env_root: &Path) -> Result<BTreeMap<String, String>> {
    if manifest.secrets.items.is_empty() {
        return Ok(BTreeMap::new());
    }

    match manifest.secrets.provider.as_str() {
        "none" => Ok(BTreeMap::new()),
        "envfile" => {
            let path = env_root.join("secrets.env");
            if !path.exists() {
                bail!("secrets.env not found at {}", path.display());
            }
            secrets::resolve_from_envfile(&path, &manifest.secrets.items)
        }
        "keychain" => {
            let service = format!("aienv.{}", manifest.id);
            secrets::resolve_from_keychain(&service, &manifest.secrets.items)
        }
        other => bail!("unknown secrets provider: {other}"),
    }
}
