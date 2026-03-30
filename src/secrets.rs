use anyhow::{anyhow, bail, Result};
use std::{collections::BTreeMap, fs, path::Path, process::Command};

pub fn resolve_from_envfile(path: &Path, items: &[String]) -> Result<BTreeMap<String, String>> {
    let raw = fs::read_to_string(path)?;
    let mut parsed = BTreeMap::new();

    for line in raw.lines().filter(|line| !line.trim().is_empty()) {
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| anyhow!("invalid envfile line: {line}"))?;
        parsed.insert(key.to_string(), value.to_string());
    }

    let mut selected = BTreeMap::new();
    for item in items {
        let value = parsed
            .get(item)
            .ok_or_else(|| anyhow!("missing secret {item} in envfile"))?;
        selected.insert(item.clone(), value.clone());
    }

    Ok(selected)
}

pub fn resolve_from_keychain(service: &str, items: &[String]) -> Result<BTreeMap<String, String>> {
    let mut selected = BTreeMap::new();

    for item in items {
        let output = Command::new("security")
            .args(["find-generic-password", "-w", "-s", service, "-a", item])
            .output()?;

        if !output.status.success() {
            bail!("missing secret {item} in keychain service {service}");
        }

        let value = String::from_utf8(output.stdout)?.trim().to_string();
        selected.insert(item.clone(), value);
    }

    Ok(selected)
}

pub fn resolve_from_provider(
    provider: SecretProvider<'_>,
    items: &[String],
) -> Result<BTreeMap<String, String>> {
    match provider {
        SecretProvider::Envfile(path) => resolve_from_envfile(path, items),
        SecretProvider::Keychain { service } => resolve_from_keychain(service, items),
    }
}

pub enum SecretProvider<'a> {
    Envfile(&'a Path),
    Keychain { service: &'a str },
}
