use anyhow::{anyhow, bail, Result};
use std::{collections::BTreeMap, fs, path::Path, process::Command};

/// On Unix, check that the envfile has no group or other permissions (mode & 0o077 == 0).
#[cfg(unix)]
fn check_file_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = fs::metadata(path)?;
    let mode = metadata.permissions().mode();
    if mode & 0o077 != 0 {
        bail!(
            "envfile {} has too-open permissions ({:o}); expected no group/other access (e.g. 0600)",
            path.display(),
            mode & 0o777
        );
    }
    Ok(())
}

#[cfg(not(unix))]
fn check_file_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

/// Parse a dotenv-style value, handling double-quoted, single-quoted, and unquoted forms.
fn parse_envfile_value(value: &str) -> String {
    if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
        let inner = &value[1..value.len() - 1];
        unescape_double_quoted(inner)
    } else if value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2 {
        let inner = &value[1..value.len() - 1];
        inner.to_string()
    } else {
        value.to_string()
    }
}

/// Handle escape sequences inside double-quoted values: \n, \t, \\, \"
fn unescape_double_quoted(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('\\') => result.push('\\'),
                Some('"') => result.push('"'),
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }
    result
}

pub fn resolve_from_envfile(path: &Path, items: &[String]) -> Result<BTreeMap<String, String>> {
    check_file_permissions(path)?;

    let raw = fs::read_to_string(path)?;
    let mut parsed = BTreeMap::new();

    for line in raw.lines() {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Strip optional `export ` prefix
        let trimmed = if let Some(rest) = trimmed.strip_prefix("export ") {
            rest
        } else {
            trimmed
        };

        let (key, value) = trimmed
            .split_once('=')
            .ok_or_else(|| anyhow!("invalid envfile line: {line}"))?;

        let key = key.trim().to_string();
        let value = parse_envfile_value(value.trim());

        parsed.insert(key, value);
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
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if stderr.is_empty() {
                bail!("failed to read secret {item} from keychain service {service}");
            }
            bail!("failed to read secret {item} from keychain service {service}: {stderr}");
        }

        let mut value = String::from_utf8(output.stdout)?;
        if value.ends_with('\n') {
            value.pop();
            if value.ends_with('\r') {
                value.pop();
            }
        }
        selected.insert(item.clone(), value);
    }

    Ok(selected)
}
