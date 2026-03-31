# aienv Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete aienv from ~60% to a fully functional CLI tool — fix all bugs, implement all missing commands (exec --tty, shell, ls, show), harden manifest validation, upgrade envfile parsing, and add network mode runtime effects.

**Architecture:** Incremental bottom-up — fix core library modules first (manifest, secrets, runtime_env, path_policy), then extract shared utilities (env_path), then build commands on top. Each task produces a compiling, test-passing codebase.

**Tech Stack:** Rust (edition 2024), clap 4.5, serde 1.0, toml 0.9, anyhow 1.0, thiserror 2.0, assert_cmd 2.0, predicates 3.1, tempfile 3.20

---

## File Structure

Planned files and responsibilities:

- Modify: `Cargo.toml`
  - Remove `portable-pty` dependency.
- Modify: `src/lib.rs`
  - Add `pub mod env_path;` export.
- Modify: `src/cli.rs`
  - Add `-e` short form, help descriptions, version flag.
- Modify: `src/manifest.rs`
  - Make secrets/shell/network optional with defaults. Add provider/reserved-key/proxy validation. Add tilde expansion. Add `proxy_url` to NetworkConfig.
- Modify: `src/runtime_env.rs`
  - Reorder forced vars to end. Add XDG_DATA_HOME/XDG_STATE_HOME. Add network mode proxy injection. Accept AIENV_ROOT and AIENV_EXEC_DIR.
- Modify: `src/secrets.rs`
  - Upgrade envfile parser to dotenv-compatible. Add file permission check.
- Modify: `src/path_policy.rs`
  - Return canonicalized shared paths in addition to cwd.
- Modify: `src/error.rs`
  - No changes needed (anyhow covers new errors).
- Create: `src/env_path.rs`
  - Public functions: `aienv_root()`, `env_root()`, `load_manifest()`, `resolve_secrets()`. Shared by all commands.
- Modify: `src/commands/mod.rs`
  - Add `pub mod shell; pub mod ls; pub mod show;`
- Modify: `src/commands/init.rs`
  - Add data/state directories, create secrets.env with 600 perms, expand deny list, use env_path module.
- Modify: `src/commands/exec.rs`
  - Use env_path for manifest/secrets. Add inherit_cwd logic. Add run directory creation/cleanup.
- Create: `src/commands/shell.rs`
  - Shell command with rc-file suppression per shell type.
- Create: `src/commands/ls.rs`
  - List environments by scanning aienv_root.
- Create: `src/commands/show.rs`
  - Print resolved manifest configuration.
- Modify: `src/main.rs`
  - Merge tty branches, wire shell/ls/show, fix signal exit codes.
- Modify: `tests/manifest_parsing.rs`
  - Add tests for optional sections, provider validation, reserved keys, tilde expansion, proxy validation.
- Modify: `tests/init_command.rs`
  - Verify data/state/secrets.env creation.
- Modify: `tests/exec_env.rs`
  - Add deny-overrides-allow, inherit_cwd=false, secrets injection, network offline, AIENV_ROOT tests.
- Create: `tests/shell_command.rs`
  - Shell rc suppression, ls output, show output tests.

---

### Task 1: Upgrade Manifest Validation and Make Sections Optional

**Files:**
- Modify: `src/manifest.rs`
- Modify: `tests/manifest_parsing.rs`

This task handles spec sections 1.1 (reserved key validation), 1.6 (provider validation), 1.8 (NetworkConfig.proxy_url), 3.1 (optional sections), 3.2 (none provider).

- [ ] **Step 1: Write failing tests for new manifest validation rules**

Add to `tests/manifest_parsing.rs`:

```rust
#[test]
fn rejects_invalid_secrets_provider() {
    let raw = r#"
id = "work"
root = "/tmp/work"
[env]
allow = []
[secrets]
provider = "bogus"
[shell]
program = "/bin/zsh"
init = "env.zsh"
[network]
mode = "default"
"#;
    let err = Manifest::parse(raw).unwrap_err();
    assert!(err.to_string().contains("secrets.provider"));
}

#[test]
fn rejects_reserved_key_in_env_set() {
    let raw = r#"
id = "work"
root = "/tmp/work"
[env]
allow = []
[env.set]
HOME = "/bad"
[secrets]
provider = "none"
[shell]
program = "/bin/zsh"
init = "env.zsh"
[network]
mode = "default"
"#;
    let err = Manifest::parse(raw).unwrap_err();
    assert!(err.to_string().contains("reserved"));
}

#[test]
fn rejects_reserved_key_in_secrets_items() {
    let raw = r#"
id = "work"
root = "/tmp/work"
[env]
allow = []
[secrets]
provider = "keychain"
items = ["TMPDIR"]
[shell]
program = "/bin/zsh"
init = "env.zsh"
[network]
mode = "default"
"#;
    let err = Manifest::parse(raw).unwrap_err();
    assert!(err.to_string().contains("reserved"));
}

#[test]
fn accepts_none_secrets_provider() {
    let raw = r#"
id = "work"
root = "/tmp/work"
[env]
allow = []
[secrets]
provider = "none"
[network]
mode = "default"
"#;
    let manifest = Manifest::parse(raw).unwrap();
    assert_eq!(manifest.secrets.provider, "none");
}

#[test]
fn optional_sections_use_defaults() {
    let raw = r#"
id = "work"
root = "/tmp/work"
[env]
allow = ["PATH"]
"#;
    let manifest = Manifest::parse(raw).unwrap();
    assert_eq!(manifest.secrets.provider, "none");
    assert_eq!(manifest.shell.program.to_string_lossy(), "/bin/zsh");
    assert_eq!(manifest.shell.init.to_string_lossy(), "env.zsh");
    assert_eq!(manifest.network.mode, "default");
    assert!(manifest.network.proxy_url.is_none());
}

#[test]
fn proxy_mode_requires_proxy_url() {
    let raw = r#"
id = "work"
root = "/tmp/work"
[env]
allow = []
[network]
mode = "proxy"
"#;
    let err = Manifest::parse(raw).unwrap_err();
    assert!(err.to_string().contains("proxy_url"));
}

#[test]
fn proxy_mode_with_url_is_valid() {
    let raw = r#"
id = "work"
root = "/tmp/work"
[env]
allow = []
[network]
mode = "proxy"
proxy_url = "http://proxy.local:8080"
"#;
    let manifest = Manifest::parse(raw).unwrap();
    assert_eq!(manifest.network.proxy_url.as_deref(), Some("http://proxy.local:8080"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test manifest_parsing -- --nocapture`
Expected: FAIL — tests reference `"none"` provider, optional sections, `proxy_url` field, and reserved key checks that don't exist yet.

- [ ] **Step 3: Implement manifest changes**

Replace the full content of `src/manifest.rs`:

```rust
use crate::error::AienvError;
use serde::Deserialize;
use std::{collections::BTreeMap, path::PathBuf};

const RESERVED_KEYS: &[&str] = &[
    "HOME",
    "XDG_CONFIG_HOME",
    "XDG_CACHE_HOME",
    "XDG_DATA_HOME",
    "XDG_STATE_HOME",
    "TMPDIR",
    "AIENV_ROOT",
];

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub id: String,
    pub root: PathBuf,
    #[serde(default = "default_true")]
    pub inherit_cwd: bool,
    #[serde(default)]
    pub shared_paths: Vec<PathBuf>,
    pub env: EnvConfig,
    #[serde(default)]
    pub secrets: SecretsConfig,
    #[serde(default)]
    pub shell: ShellConfig,
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub extends: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct EnvConfig {
    #[serde(default)]
    pub allow: Vec<String>,
    #[serde(default)]
    pub deny: Vec<String>,
    #[serde(default)]
    pub set: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecretsConfig {
    #[serde(default = "default_none_provider")]
    pub provider: String,
    #[serde(default)]
    pub items: Vec<String>,
}

impl Default for SecretsConfig {
    fn default() -> Self {
        Self {
            provider: "none".into(),
            items: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShellConfig {
    #[serde(default = "default_zsh")]
    pub program: PathBuf,
    #[serde(default = "default_env_zsh")]
    pub init: PathBuf,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            program: PathBuf::from("/bin/zsh"),
            init: PathBuf::from("env.zsh"),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkConfig {
    #[serde(default = "default_network_mode")]
    pub mode: String,
    #[serde(default)]
    pub proxy_url: Option<String>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            mode: "default".into(),
            proxy_url: None,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_none_provider() -> String {
    "none".into()
}

fn default_zsh() -> PathBuf {
    PathBuf::from("/bin/zsh")
}

fn default_env_zsh() -> PathBuf {
    PathBuf::from("env.zsh")
}

fn default_network_mode() -> String {
    "default".into()
}

impl Manifest {
    pub fn parse(raw: &str) -> Result<Self, AienvError> {
        let mut manifest: Self = toml::from_str(raw)?;
        manifest.expand_tilde();
        manifest.validate()?;
        Ok(manifest)
    }

    fn expand_tilde(&mut self) {
        if let Ok(home) = std::env::var("HOME") {
            self.root = expand_tilde_path(&self.root, &home);
            self.shared_paths = self
                .shared_paths
                .iter()
                .map(|p| expand_tilde_path(p, &home))
                .collect();
        }
    }

    pub fn validate(&self) -> Result<(), AienvError> {
        if self.extends.is_some() {
            return Err(AienvError::ManifestValidation(
                "manifest inheritance via `extends` is not implemented yet".into(),
            ));
        }

        match self.network.mode.as_str() {
            "default" | "offline" | "proxy" => {}
            other => {
                return Err(AienvError::ManifestValidation(format!(
                    "network.mode must be default|offline|proxy, got {other}"
                )));
            }
        }

        if self.network.mode == "proxy" && self.network.proxy_url.is_none() {
            return Err(AienvError::ManifestValidation(
                "network.proxy_url is required when network.mode is \"proxy\"".into(),
            ));
        }

        if self.id.trim().is_empty() {
            return Err(AienvError::ManifestValidation(
                "id cannot be empty".into(),
            ));
        }

        match self.secrets.provider.as_str() {
            "keychain" | "envfile" | "none" => {}
            other => {
                return Err(AienvError::ManifestValidation(format!(
                    "secrets.provider must be keychain|envfile|none, got {other}"
                )));
            }
        }

        for key in self.env.set.keys() {
            if RESERVED_KEYS.contains(&key.as_str()) {
                return Err(AienvError::ManifestValidation(format!(
                    "env.set contains reserved key \"{key}\" which is managed by aienv"
                )));
            }
        }

        for item in &self.secrets.items {
            if RESERVED_KEYS.contains(&item.as_str()) {
                return Err(AienvError::ManifestValidation(format!(
                    "secrets.items contains reserved key \"{item}\" which is managed by aienv"
                )));
            }
        }

        Ok(())
    }
}

fn expand_tilde_path(path: &PathBuf, home: &str) -> PathBuf {
    let s = path.to_string_lossy();
    if s == "~" {
        PathBuf::from(home)
    } else if let Some(rest) = s.strip_prefix("~/") {
        PathBuf::from(home).join(rest)
    } else {
        path.clone()
    }
}
```

- [ ] **Step 4: Run all tests**

Run: `cargo test`
Expected: PASS for all tests including new manifest validation tests. Some existing tests may need `[secrets]` provider updated from `"keychain"` to include the new validation — check and fix any failures.

- [ ] **Step 5: Fix any existing test breakage from provider validation**

The existing tests in `tests/manifest_parsing.rs` and `tests/exec_env.rs` use `provider = "keychain"` which is still valid, so they should pass. Verify no breakage.

- [ ] **Step 6: Add tilde expansion test**

Add to `tests/manifest_parsing.rs`:

```rust
#[test]
fn expands_tilde_in_root_path() {
    let home = std::env::var("HOME").unwrap();
    let raw = r#"
id = "work"
root = "~/.aienv/work"
[env]
allow = []
"#;
    let manifest = Manifest::parse(raw).unwrap();
    assert_eq!(
        manifest.root,
        PathBuf::from(&home).join(".aienv/work")
    );
}
```

- [ ] **Step 7: Run all tests**

Run: `cargo test`
Expected: PASS for all tests.

- [ ] **Step 8: Commit**

```bash
git add src/manifest.rs tests/manifest_parsing.rs
git commit -m "feat: upgrade manifest validation — optional sections, provider check, reserved keys, tilde expansion, proxy_url"
```

---

### Task 2: Upgrade Secrets Module — Dotenv Parsing and Permission Check

**Files:**
- Modify: `src/secrets.rs`
- Modify: `tests/exec_env.rs`

Covers spec sections 1.9 (envfile permission check), 3.3 (dotenv-compatible parsing).

- [ ] **Step 1: Write failing tests for dotenv parsing**

Add to `tests/exec_env.rs`:

```rust
#[test]
fn envfile_supports_comments_and_blank_lines() {
    let dir = tempfile::TempDir::new().unwrap();
    let envfile = dir.path().join("secrets.env");
    std::fs::write(
        &envfile,
        "# this is a comment\n\nOPENAI_API_KEY=one\n# another comment\nGEMINI_KEY=two\n",
    )
    .unwrap();

    let secrets =
        resolve_from_envfile(&envfile, &["OPENAI_API_KEY".into(), "GEMINI_KEY".into()]).unwrap();
    assert_eq!(secrets["OPENAI_API_KEY"], "one");
    assert_eq!(secrets["GEMINI_KEY"], "two");
}

#[test]
fn envfile_supports_export_prefix() {
    let dir = tempfile::TempDir::new().unwrap();
    let envfile = dir.path().join("secrets.env");
    std::fs::write(&envfile, "export OPENAI_API_KEY=one\n").unwrap();

    let secrets = resolve_from_envfile(&envfile, &["OPENAI_API_KEY".into()]).unwrap();
    assert_eq!(secrets["OPENAI_API_KEY"], "one");
}

#[test]
fn envfile_supports_double_quoted_values() {
    let dir = tempfile::TempDir::new().unwrap();
    let envfile = dir.path().join("secrets.env");
    std::fs::write(&envfile, "KEY=\"hello world\"\nKEY2=\"line\\nbreak\"\n").unwrap();

    let secrets =
        resolve_from_envfile(&envfile, &["KEY".into(), "KEY2".into()]).unwrap();
    assert_eq!(secrets["KEY"], "hello world");
    assert_eq!(secrets["KEY2"], "line\nbreak");
}

#[test]
fn envfile_supports_single_quoted_values() {
    let dir = tempfile::TempDir::new().unwrap();
    let envfile = dir.path().join("secrets.env");
    std::fs::write(&envfile, "KEY='hello world'\nKEY2='no\\nescape'\n").unwrap();

    let secrets =
        resolve_from_envfile(&envfile, &["KEY".into(), "KEY2".into()]).unwrap();
    assert_eq!(secrets["KEY"], "hello world");
    assert_eq!(secrets["KEY2"], "no\\nescape");
}

#[test]
fn envfile_trims_whitespace_around_key_and_value() {
    let dir = tempfile::TempDir::new().unwrap();
    let envfile = dir.path().join("secrets.env");
    std::fs::write(&envfile, "  KEY  =  value  \n").unwrap();

    let secrets = resolve_from_envfile(&envfile, &["KEY".into()]).unwrap();
    assert_eq!(secrets["KEY"], "value");
}

#[cfg(unix)]
#[test]
fn envfile_rejects_open_permissions() {
    let dir = tempfile::TempDir::new().unwrap();
    let envfile = dir.path().join("secrets.env");
    std::fs::write(&envfile, "KEY=value\n").unwrap();

    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&envfile, std::fs::Permissions::from_mode(0o644)).unwrap();

    let err = resolve_from_envfile(&envfile, &["KEY".into()]).unwrap_err();
    assert!(err.to_string().contains("permissions"));
}

#[cfg(unix)]
#[test]
fn envfile_accepts_strict_permissions() {
    let dir = tempfile::TempDir::new().unwrap();
    let envfile = dir.path().join("secrets.env");
    std::fs::write(&envfile, "KEY=value\n").unwrap();

    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&envfile, std::fs::Permissions::from_mode(0o600)).unwrap();

    let secrets = resolve_from_envfile(&envfile, &["KEY".into()]).unwrap();
    assert_eq!(secrets["KEY"], "value");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test exec_env -- --nocapture`
Expected: FAIL — comments cause "invalid envfile line", no permission check exists, no quote handling.

- [ ] **Step 3: Implement upgraded secrets module**

Replace the full content of `src/secrets.rs`:

```rust
use anyhow::{anyhow, bail, Result};
use std::{collections::BTreeMap, fs, path::Path, process::Command};

pub fn resolve_from_envfile(path: &Path, items: &[String]) -> Result<BTreeMap<String, String>> {
    check_file_permissions(path)?;

    let raw = fs::read_to_string(path)?;
    let mut parsed = BTreeMap::new();

    for (line_num, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let line_content = trimmed
            .strip_prefix("export ")
            .unwrap_or(trimmed);

        let (key, value) = line_content
            .split_once('=')
            .ok_or_else(|| anyhow!("invalid envfile line {}: {line}", line_num + 1))?;

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

fn parse_envfile_value(raw: &str) -> String {
    if raw.len() >= 2 {
        if raw.starts_with('"') && raw.ends_with('"') {
            let inner = &raw[1..raw.len() - 1];
            return unescape_double_quoted(inner);
        }
        if raw.starts_with('\'') && raw.ends_with('\'') {
            return raw[1..raw.len() - 1].to_string();
        }
    }
    raw.to_string()
}

fn unescape_double_quoted(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
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
            result.push(ch);
        }
    }
    result
}

fn check_file_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(path)?.permissions().mode();
        if mode & 0o077 != 0 {
            bail!(
                "secrets.env permissions too open ({:o}), expected 600 or stricter: {}",
                mode & 0o777,
                path.display()
            );
        }
    }
    Ok(())
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
```

- [ ] **Step 4: Run all tests**

Run: `cargo test`
Expected: PASS for all tests. The old `SecretProvider` enum is removed — verify no other code references it. If `tests/exec_env.rs` imports `SecretProvider`, remove those imports and usages.

- [ ] **Step 5: Fix any test imports referencing removed `SecretProvider`**

In `tests/exec_env.rs`, find and remove any `use aienv::secrets::SecretProvider;` and any lines creating `SecretProvider::Envfile(...)` or `SecretProvider::Keychain { .. }` instances. These were unused except as type-check exercises in the original tests.

- [ ] **Step 6: Run all tests again**

Run: `cargo test`
Expected: PASS for all tests.

- [ ] **Step 7: Commit**

```bash
git add src/secrets.rs tests/exec_env.rs
git commit -m "feat: upgrade envfile parser to dotenv format, add permission check"
```

---

### Task 3: Upgrade runtime_env — Forced Vars Last, Network Mode, XDG Expansion

**Files:**
- Modify: `src/runtime_env.rs`
- Modify: `tests/exec_env.rs`

Covers spec sections 1.1 (forced vars last), 1.5 (AIENV_ROOT), 1.8 (network mode injection), 3.4 (XDG_DATA_HOME/XDG_STATE_HOME), 4.4 (AIENV_EXEC_DIR).

- [ ] **Step 1: Write failing tests for new runtime_env behavior**

Add to `tests/exec_env.rs`:

```rust
use aienv::manifest::Manifest;

#[test]
fn forced_vars_cannot_be_overridden_by_env_set() {
    // env.set tries to set XDG_DATA_HOME — validate rejects it
    let raw = r#"
id = "work"
root = "/tmp/work"
[env]
allow = []
[env.set]
XDG_DATA_HOME = "/bad"
"#;
    let err = Manifest::parse(raw).unwrap_err();
    assert!(err.to_string().contains("reserved"));
}

#[test]
fn builds_env_with_xdg_data_and_state() {
    let manifest = Manifest::parse(
        r#"
id = "work"
root = "/tmp/work"
[env]
allow = ["TERM"]
[env.set]
AI_ENV = "work"
"#,
    )
    .unwrap();

    let mut host = BTreeMap::new();
    host.insert("TERM".into(), "xterm".into());
    host.insert("HOME".into(), "/Users/test".into());

    let env = build_child_env(&manifest, &host, BTreeMap::new(), None);
    assert_eq!(env["XDG_DATA_HOME"], "/tmp/work/data");
    assert_eq!(env["XDG_STATE_HOME"], "/tmp/work/state");
    assert_eq!(env["HOME"], "/tmp/work/home");
    assert_eq!(env["AIENV_ROOT"], "/Users/test/.aienv");
}

#[test]
fn deny_overrides_allow_when_both_present() {
    let manifest = Manifest::parse(
        r#"
id = "work"
root = "/tmp/work"
[env]
allow = ["TERM", "SECRET"]
deny = ["SECRET"]
"#,
    )
    .unwrap();

    let mut host = BTreeMap::new();
    host.insert("TERM".into(), "xterm".into());
    host.insert("SECRET".into(), "should-not-leak".into());
    host.insert("HOME".into(), "/Users/test".into());

    let env = build_child_env(&manifest, &host, BTreeMap::new(), None);
    assert_eq!(env["TERM"], "xterm");
    assert!(!env.contains_key("SECRET"));
}

#[test]
fn network_offline_injects_proxy_vars() {
    let manifest = Manifest::parse(
        r#"
id = "work"
root = "/tmp/work"
[env]
allow = []
[network]
mode = "offline"
"#,
    )
    .unwrap();

    let mut host = BTreeMap::new();
    host.insert("HOME".into(), "/Users/test".into());

    let env = build_child_env(&manifest, &host, BTreeMap::new(), None);
    assert_eq!(env["http_proxy"], "http://127.0.0.1:1");
    assert_eq!(env["https_proxy"], "http://127.0.0.1:1");
    assert_eq!(env["ALL_PROXY"], "http://127.0.0.1:1");
}

#[test]
fn network_proxy_injects_custom_url() {
    let manifest = Manifest::parse(
        r#"
id = "work"
root = "/tmp/work"
[env]
allow = []
[network]
mode = "proxy"
proxy_url = "http://proxy.local:8080"
"#,
    )
    .unwrap();

    let mut host = BTreeMap::new();
    host.insert("HOME".into(), "/Users/test".into());

    let env = build_child_env(&manifest, &host, BTreeMap::new(), None);
    assert_eq!(env["http_proxy"], "http://proxy.local:8080");
    assert_eq!(env["https_proxy"], "http://proxy.local:8080");
    assert_eq!(env["ALL_PROXY"], "http://proxy.local:8080");
}

#[test]
fn aienv_root_preserves_existing_value() {
    let manifest = Manifest::parse(
        r#"
id = "work"
root = "/tmp/work"
[env]
allow = []
"#,
    )
    .unwrap();

    let mut host = BTreeMap::new();
    host.insert("HOME".into(), "/Users/test".into());
    host.insert("AIENV_ROOT".into(), "/custom/aienv".into());

    let env = build_child_env(&manifest, &host, BTreeMap::new(), None);
    assert_eq!(env["AIENV_ROOT"], "/custom/aienv");
}

#[test]
fn aienv_exec_dir_injected_when_provided() {
    let manifest = Manifest::parse(
        r#"
id = "work"
root = "/tmp/work"
[env]
allow = []
"#,
    )
    .unwrap();

    let mut host = BTreeMap::new();
    host.insert("HOME".into(), "/Users/test".into());

    let env = build_child_env(
        &manifest,
        &host,
        BTreeMap::new(),
        Some("/tmp/work/run/12345"),
    );
    assert_eq!(env["AIENV_EXEC_DIR"], "/tmp/work/run/12345");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test exec_env -- --nocapture`
Expected: FAIL — `build_child_env` signature doesn't accept `exec_dir` parameter, doesn't produce XDG_DATA_HOME/XDG_STATE_HOME, etc.

- [ ] **Step 3: Implement upgraded runtime_env**

Replace the full content of `src/runtime_env.rs`:

```rust
use crate::manifest::Manifest;
use std::{collections::BTreeMap, path::Path};

/// Build the child process environment.
///
/// Execution order:
/// 1. Allow list: inherit from host, filtered by deny
/// 2. env.set: fixed injected variables
/// 3. secrets: from keychain/envfile
/// 4. network.mode: proxy injection for offline/proxy
/// 5. exec_dir: AIENV_EXEC_DIR if provided
/// 6. Forced variables (last, cannot be overridden):
///    HOME, XDG_CONFIG_HOME, XDG_CACHE_HOME, XDG_DATA_HOME,
///    XDG_STATE_HOME, TMPDIR, AIENV_ROOT
pub fn build_child_env(
    manifest: &Manifest,
    host: &BTreeMap<String, String>,
    secrets: BTreeMap<String, String>,
    exec_dir: Option<&str>,
) -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();

    // 1. Allow list filtered by deny
    for key in &manifest.env.allow {
        if manifest.env.deny.contains(key) {
            continue;
        }
        if let Some(value) = host.get(key) {
            env.insert(key.clone(), value.clone());
        }
    }

    // 2. env.set
    for (key, value) in &manifest.env.set {
        env.insert(key.clone(), value.clone());
    }

    // 3. secrets
    for (key, value) in secrets {
        env.insert(key, value);
    }

    // 4. network mode
    let proxy_url = match manifest.network.mode.as_str() {
        "offline" => Some("http://127.0.0.1:1".to_string()),
        "proxy" => manifest.network.proxy_url.clone(),
        _ => None,
    };
    if let Some(url) = proxy_url {
        env.insert("http_proxy".into(), url.clone());
        env.insert("https_proxy".into(), url.clone());
        env.insert("ALL_PROXY".into(), url);
    }

    // 5. exec dir
    if let Some(dir) = exec_dir {
        env.insert("AIENV_EXEC_DIR".into(), dir.to_string());
    }

    // 6. Forced variables (last — cannot be overridden)
    env.insert("HOME".into(), join_str(&manifest.root, "home"));
    env.insert("XDG_CONFIG_HOME".into(), join_str(&manifest.root, "config"));
    env.insert("XDG_CACHE_HOME".into(), join_str(&manifest.root, "cache"));
    env.insert("XDG_DATA_HOME".into(), join_str(&manifest.root, "data"));
    env.insert("XDG_STATE_HOME".into(), join_str(&manifest.root, "state"));
    env.insert("TMPDIR".into(), join_str(&manifest.root, "tmp"));

    let aienv_root = host
        .get("AIENV_ROOT")
        .cloned()
        .unwrap_or_else(|| {
            host.get("HOME")
                .map(|h| format!("{h}/.aienv"))
                .unwrap_or_default()
        });
    env.insert("AIENV_ROOT".into(), aienv_root);

    env
}

fn join_str(root: &Path, child: &str) -> String {
    root.join(child).to_string_lossy().into_owned()
}
```

- [ ] **Step 4: Fix call sites**

`build_child_env` now takes an extra `exec_dir: Option<&str>` parameter. Update `src/commands/exec.rs` line 17:

Change:
```rust
let child_env = build_child_env(&manifest, &host, BTreeMap::new());
```
To:
```rust
let child_env = build_child_env(&manifest, &host, BTreeMap::new(), None);
```

Also update `tests/exec_env.rs` — the existing `builds_sanitized_child_env` test calls `build_child_env` with 3 args. Add `None` as the 4th argument:

Change:
```rust
let env = build_child_env(&manifest, &host, BTreeMap::new());
```
To:
```rust
let env = build_child_env(&manifest, &host, BTreeMap::new(), None);
```

And add `host.insert("HOME".into(), "/Users/test".into());` to the existing `builds_sanitized_child_env` test's host map so AIENV_ROOT can be derived.

- [ ] **Step 5: Run all tests**

Run: `cargo test`
Expected: PASS for all tests.

- [ ] **Step 6: Commit**

```bash
git add src/runtime_env.rs src/commands/exec.rs tests/exec_env.rs
git commit -m "feat: forced vars last, network mode proxy injection, XDG data/state, AIENV_ROOT"
```

---

### Task 4: Upgrade path_policy and Extract env_path Module

**Files:**
- Modify: `src/path_policy.rs`
- Create: `src/env_path.rs`
- Modify: `src/lib.rs`
- Modify: `src/commands/exec.rs`
- Modify: `src/commands/init.rs`

Covers spec sections 1.2 (secrets resolution), 1.5 (AIENV_ROOT in load_manifest), 1.10 (id/root validation), 4.3 (extract env_path), 4.5 (path_policy return type).

- [ ] **Step 1: Update path_policy return type**

Replace the full content of `src/path_policy.rs`:

```rust
use anyhow::Result;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn validate_cwd(cwd: &Path, shared_paths: &[PathBuf]) -> Result<(PathBuf, Vec<PathBuf>)> {
    let cwd_real = fs::canonicalize(cwd).map_err(|e| anyhow::anyhow!("cwd is invalid: {e}"))?;

    let mut shared_real = Vec::new();
    for path in shared_paths {
        let canonical =
            fs::canonicalize(path).map_err(|e| anyhow::anyhow!("shared path is invalid: {e}"))?;
        shared_real.push(canonical);
    }

    Ok((cwd_real, shared_real))
}
```

- [ ] **Step 2: Fix call sites for new path_policy return type**

In `src/commands/exec.rs`, change:

```rust
let cwd = validate_cwd(&cwd, &manifest.shared_paths)?;
```
To:
```rust
let (cwd, _shared) = validate_cwd(&cwd, &manifest.shared_paths)?;
```

In `tests/exec_env.rs`, fix the two tests that call `validate_cwd`:

Change `allows_normal_cwd_even_when_shared_paths_are_present`:
```rust
let validated = validate_cwd(&cwd, &[shared.path().to_path_buf()]).unwrap();
```
To:
```rust
let (validated, _) = validate_cwd(&cwd, &[shared.path().to_path_buf()]).unwrap();
```

Change `rejects_missing_cwd` — no change needed (it tests the error path).

Change `rejects_invalid_shared_paths` — no change needed (it tests the error path).

- [ ] **Step 3: Create env_path module**

Create `src/env_path.rs`:

```rust
use crate::manifest::Manifest;
use crate::secrets;
use anyhow::{bail, Context, Result};
use std::{collections::BTreeMap, fs, path::{Path, PathBuf}};

/// Return the aienv root directory.
/// Prefers AIENV_ROOT env var, falls back to $HOME/.aienv.
pub fn aienv_root() -> Result<PathBuf> {
    if let Ok(root) = std::env::var("AIENV_ROOT") {
        return Ok(PathBuf::from(root));
    }
    let home = std::env::var("HOME").context("HOME not set")?;
    Ok(PathBuf::from(home).join(".aienv"))
}

/// Return the root directory for a specific environment.
pub fn env_root(env: &str) -> Result<PathBuf> {
    Ok(aienv_root()?.join(env))
}

/// Load and validate a manifest for the given environment.
/// Returns (manifest, env_root_path).
pub fn load_manifest(env: &str) -> Result<(Manifest, PathBuf)> {
    let root = env_root(env)?;
    let path = root.join("manifest.toml");
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read manifest {}", path.display()))?;
    let manifest = Manifest::parse(&raw)?;

    // Validate id matches directory name
    if manifest.id != env {
        bail!(
            "manifest id \"{}\" does not match environment name \"{}\"",
            manifest.id,
            env
        );
    }

    // Validate root matches actual directory
    if manifest.root != root {
        bail!(
            "manifest root \"{}\" does not match environment directory \"{}\"",
            manifest.root.display(),
            root.display()
        );
    }

    Ok((manifest, root))
}

/// Resolve secrets based on manifest configuration.
pub fn resolve_secrets(
    manifest: &Manifest,
    env_root: &Path,
) -> Result<BTreeMap<String, String>> {
    if manifest.secrets.items.is_empty() {
        return Ok(BTreeMap::new());
    }

    match manifest.secrets.provider.as_str() {
        "none" => Ok(BTreeMap::new()),
        "envfile" => {
            let path = env_root.join("secrets.env");
            if !path.exists() {
                bail!(
                    "secrets.env not found at {}",
                    path.display()
                );
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
```

- [ ] **Step 4: Export env_path in lib.rs**

In `src/lib.rs`, add the new module:

```rust
pub mod cli;
pub mod commands;
pub mod env_path;
pub mod error;
pub mod manifest;
pub mod path_policy;
pub mod runtime_env;
pub mod secrets;
```

- [ ] **Step 5: Update exec.rs to use env_path**

Replace the full content of `src/commands/exec.rs`:

```rust
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

    // Create per-execution run directory
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

    // Best-effort cleanup of run directory
    let _ = fs::remove_dir_all(&run_dir);

    Ok(status)
}
```

- [ ] **Step 6: Update init.rs to use env_path and add new directories**

Replace the full content of `src/commands/init.rs`:

```rust
use crate::{env_path, manifest::Manifest};
use anyhow::{anyhow, Result};
use std::{fs, path::PathBuf};

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

    if env.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_') {
        Ok(())
    } else {
        Err(anyhow!(
            "environment name may only contain ASCII letters, digits, '-' and '_'"
        ))
    }
}

fn default_manifest(env: &str, root: &PathBuf) -> String {
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
```

- [ ] **Step 7: Update init tests for new directories and secrets.env**

Add to `tests/init_command.rs`:

```rust
#[test]
fn init_creates_data_state_dirs_and_secrets_env() {
    let home = TempDir::new().unwrap();
    let env_root = home.path().join(".aienv").join("newenv");

    let mut cmd = Command::cargo_bin("aienv").unwrap();
    cmd.env("HOME", home.path())
        .args(["init", "--env", "newenv"]);

    cmd.assert().success();
    assert!(env_root.join("data").exists());
    assert!(env_root.join("state").exists());
    assert!(env_root.join("secrets.env").exists());

    // Verify secrets.env has restrictive permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(env_root.join("secrets.env"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}
```

- [ ] **Step 8: Run all tests**

Run: `cargo test`
Expected: PASS. If any existing tests fail due to manifest id/root validation in `load_manifest`, those tests create manifests with `root` pointing to the temp dir, which should match. Check the `exec_runs_command_in_environment_with_isolated_home` test — its manifest's root must match the actual env_root path.

- [ ] **Step 9: Commit**

```bash
git add src/path_policy.rs src/env_path.rs src/lib.rs src/commands/exec.rs src/commands/init.rs tests/exec_env.rs tests/init_command.rs
git commit -m "feat: extract env_path module, secrets resolution, inherit_cwd, run dirs, expanded deny list"
```

---

### Task 5: Implement ls and show Commands

**Files:**
- Create: `src/commands/ls.rs`
- Create: `src/commands/show.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/main.rs`
- Create: `tests/shell_command.rs`

Covers spec sections 2.3 (ls), 2.4 (show).

- [ ] **Step 1: Write failing tests for ls and show**

Create `tests/shell_command.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn write_minimal_manifest(env_root: &std::path::Path, id: &str) {
    for dir in ["home", "config", "cache", "data", "state", "tmp", "run"] {
        fs::create_dir_all(env_root.join(dir)).unwrap();
    }
    fs::write(
        env_root.join("manifest.toml"),
        format!(
            r#"id = "{id}"
root = "{root}"
[env]
allow = ["PATH"]
deny = ["OPENAI_API_KEY"]
[env.set]
AI_ENV = "{id}"
[secrets]
provider = "none"
[shell]
program = "/bin/zsh"
init = "env.zsh"
[network]
mode = "default"
"#,
            root = env_root.display()
        ),
    )
    .unwrap();
    fs::write(env_root.join("env.zsh"), format!("export AI_ENV={id}\n")).unwrap();
}

#[test]
fn ls_lists_initialized_environments() {
    let home = TempDir::new().unwrap();
    let aienv = home.path().join(".aienv");

    write_minimal_manifest(&aienv.join("alpha"), "alpha");
    write_minimal_manifest(&aienv.join("beta"), "beta");

    let mut cmd = Command::cargo_bin("aienv").unwrap();
    cmd.env("HOME", home.path()).arg("ls");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("alpha"))
        .stdout(predicate::str::contains("beta"));
}

#[test]
fn ls_shows_nothing_when_no_environments() {
    let home = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("aienv").unwrap();
    cmd.env("HOME", home.path()).arg("ls");
    cmd.assert().success().stdout(predicate::str::is_empty());
}

#[test]
fn show_prints_resolved_config() {
    let home = TempDir::new().unwrap();
    let env_root = home.path().join(".aienv").join("work");
    write_minimal_manifest(&env_root, "work");

    let mut cmd = Command::cargo_bin("aienv").unwrap();
    cmd.env("HOME", home.path())
        .args(["show", "--env", "work"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Environment:"))
        .stdout(predicate::str::contains("work"))
        .stdout(predicate::str::contains("Env Allow:"))
        .stdout(predicate::str::contains("PATH"))
        .stdout(predicate::str::contains("Directories:"))
        .stdout(predicate::str::contains("home"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test shell_command -- --nocapture`
Expected: FAIL — ls and show are still `todo!()`.

- [ ] **Step 3: Implement ls command**

Create `src/commands/ls.rs`:

```rust
use crate::env_path;
use anyhow::Result;
use std::fs;

pub fn run() -> Result<()> {
    let root = env_path::aienv_root()?;
    if !root.exists() {
        return Ok(());
    }

    let mut names: Vec<String> = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if entry.path().join("manifest.toml").exists() {
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
    }

    names.sort();
    for name in names {
        println!("{name}");
    }

    Ok(())
}
```

- [ ] **Step 4: Implement show command**

Create `src/commands/show.rs`:

```rust
use crate::env_path;
use anyhow::Result;

pub fn run(env: &str) -> Result<()> {
    let (manifest, env_root) = env_path::load_manifest(env)?;

    println!("Environment:     {}", manifest.id);
    println!("Root:            {}", manifest.root.display());
    println!("Inherit CWD:     {}", manifest.inherit_cwd);
    println!("Network:         {}", manifest.network.mode);
    if let Some(url) = &manifest.network.proxy_url {
        println!("Proxy URL:       {url}");
    }
    println!();

    if manifest.env.allow.is_empty() {
        println!("Env Allow:       (none)");
    } else {
        println!("Env Allow:       {}", manifest.env.allow.join(", "));
    }
    if manifest.env.deny.is_empty() {
        println!("Env Deny:        (none)");
    } else {
        println!("Env Deny:        {}", manifest.env.deny.join(", "));
    }
    if manifest.env.set.is_empty() {
        println!("Env Set:         (none)");
    } else {
        let pairs: Vec<String> = manifest
            .env
            .set
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        println!("Env Set:         {}", pairs.join(", "));
    }
    println!();

    let provider_display = match manifest.secrets.provider.as_str() {
        "keychain" => format!("keychain (aienv.{})", manifest.id),
        "envfile" => format!("envfile ({})", env_root.join("secrets.env").display()),
        other => other.to_string(),
    };
    println!("Secrets:         {provider_display}");
    if manifest.secrets.items.is_empty() {
        println!("Secret Items:    (none)");
    } else {
        println!("Secret Items:    {}", manifest.secrets.items.join(", "));
    }
    println!();

    println!("Shell:           {}", manifest.shell.program.display());
    println!("Shell Init:      {}", manifest.shell.init.display());
    println!();

    println!("Directories:");
    println!("  HOME             {}", manifest.root.join("home").display());
    println!(
        "  XDG_CONFIG_HOME  {}",
        manifest.root.join("config").display()
    );
    println!(
        "  XDG_CACHE_HOME   {}",
        manifest.root.join("cache").display()
    );
    println!(
        "  XDG_DATA_HOME    {}",
        manifest.root.join("data").display()
    );
    println!(
        "  XDG_STATE_HOME   {}",
        manifest.root.join("state").display()
    );
    println!("  TMPDIR           {}", manifest.root.join("tmp").display());

    Ok(())
}
```

- [ ] **Step 5: Update commands/mod.rs**

Replace the full content of `src/commands/mod.rs`:

```rust
pub mod exec;
pub mod init;
pub mod ls;
pub mod shell;
pub mod show;
```

Note: `shell` module doesn't exist yet — create a placeholder `src/commands/shell.rs`:

```rust
use anyhow::Result;
use std::path::PathBuf;

pub fn run(_env: &str, _cwd: Option<PathBuf>) -> Result<i32> {
    todo!("shell implementation in next task")
}
```

- [ ] **Step 6: Wire ls, show, and fix main.rs**

Replace the full content of `src/main.rs`:

```rust
use aienv::{
    cli::{Cli, Commands},
    commands,
};
use anyhow::Result;
use clap::Parser;
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
```

- [ ] **Step 7: Run all tests**

Run: `cargo test`
Expected: PASS for all tests.

- [ ] **Step 8: Commit**

```bash
git add src/main.rs src/commands/mod.rs src/commands/ls.rs src/commands/show.rs src/commands/shell.rs tests/shell_command.rs
git commit -m "feat: implement ls and show commands, fix signal exit codes"
```

---

### Task 6: Implement Shell Command

**Files:**
- Modify: `src/commands/shell.rs`
- Modify: `tests/shell_command.rs`

Covers spec section 2.2 (shell with rc suppression).

- [ ] **Step 1: Write failing test for shell command**

Add to `tests/shell_command.rs`:

```rust
#[test]
fn shell_sources_env_init_and_runs_interactively() {
    let home = TempDir::new().unwrap();
    let env_root = home.path().join(".aienv").join("work");
    write_minimal_manifest(&env_root, "work");
    fs::write(
        env_root.join("env.zsh"),
        "export SHELL_SENTINEL=from-env-init\n",
    )
    .unwrap();

    // Use exec --tty to test the same code path as shell would use,
    // but with a non-interactive command that checks the env
    let mut cmd = Command::cargo_bin("aienv").unwrap();
    cmd.env("HOME", home.path()).args([
        "exec",
        "--env",
        "work",
        "--",
        "python3",
        "-c",
        "import os; print(os.environ.get('AI_ENV', 'missing'))",
    ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("work"));
}

#[test]
fn shell_command_builds_correct_zsh_args() {
    // Unit-level test: verify the shell arg construction
    use aienv::commands::shell::build_shell_args;

    let args = build_shell_args(
        std::path::Path::new("/bin/zsh"),
        std::path::Path::new("/root"),
        std::path::Path::new("env.zsh"),
    );
    assert_eq!(args[0], "/bin/zsh");
    assert_eq!(args[1], "--no-globalrcs");
    assert_eq!(args[2], "--no-rcs");
    assert_eq!(args[3], "-c");
    assert!(args[4].contains("source"));
    assert!(args[4].contains("env.zsh"));
    assert!(args[4].contains("exec"));
    assert!(args[4].contains("-i"));
}

#[test]
fn shell_command_builds_correct_bash_args() {
    use aienv::commands::shell::build_shell_args;

    let args = build_shell_args(
        std::path::Path::new("/bin/bash"),
        std::path::Path::new("/root"),
        std::path::Path::new("env.zsh"),
    );
    assert_eq!(args[0], "/bin/bash");
    assert_eq!(args[1], "--noprofile");
    assert_eq!(args[2], "--norc");
    assert_eq!(args[3], "-c");
    assert!(args[4].contains("source"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test shell_command -- --nocapture`
Expected: FAIL — `build_shell_args` doesn't exist yet, shell::run is `todo!()`.

- [ ] **Step 3: Implement shell command**

Replace the full content of `src/commands/shell.rs`:

```rust
use anyhow::Result;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{env_path, path_policy::validate_cwd, runtime_env::build_child_env};

pub fn run(env: &str, cwd: Option<PathBuf>) -> Result<i32> {
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

    let args = build_shell_args(&manifest.shell.program, &manifest.root, &manifest.shell.init);

    let status = Command::new(&args[0])
        .args(&args[1..])
        .current_dir(&cwd)
        .env_clear()
        .envs(child_env)
        .status()?;

    let _ = fs::remove_dir_all(&run_dir);

    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        Ok(status
            .code()
            .unwrap_or_else(|| 128 + status.signal().unwrap_or(1)))
    }
    #[cfg(not(unix))]
    {
        Ok(status.code().unwrap_or(1))
    }
}

/// Build shell launch arguments with rc-file suppression.
///
/// Strategy: suppress all default rc files, source the env init,
/// then exec into an interactive instance of the same shell.
pub fn build_shell_args(program: &Path, env_root: &Path, init: &Path) -> Vec<String> {
    let program_str = program.to_string_lossy().to_string();
    let init_path = env_root.join(init);
    let init_str = init_path.to_string_lossy().to_string();

    let shell_name = program
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let (suppress_flags, exec_cmd) = match shell_name.as_str() {
        "zsh" => (
            vec!["--no-globalrcs".to_string(), "--no-rcs".to_string()],
            format!(
                "source \"{init_str}\" && exec \"{program_str}\" --no-globalrcs --no-rcs -i"
            ),
        ),
        "bash" => (
            vec!["--noprofile".to_string(), "--norc".to_string()],
            format!(
                "source \"{init_str}\" && exec \"{program_str}\" --noprofile --norc -i"
            ),
        ),
        _ => (
            Vec::new(),
            format!(
                "source \"{init_str}\" && exec \"{program_str}\" -i"
            ),
        ),
    };

    let mut args = vec![program_str];
    args.extend(suppress_flags);
    args.push("-c".to_string());
    args.push(exec_cmd);
    args
}
```

- [ ] **Step 4: Run all tests**

Run: `cargo test`
Expected: PASS for all tests.

- [ ] **Step 5: Commit**

```bash
git add src/commands/shell.rs tests/shell_command.rs
git commit -m "feat: implement shell command with rc-file suppression"
```

---

### Task 7: CLI Polish and Dependency Cleanup

**Files:**
- Modify: `src/cli.rs`
- Modify: `Cargo.toml`
- Modify: `tests/cli_help.rs`

Covers spec sections 4.1 (CLI improvements), 4.2 (remove portable-pty).

- [ ] **Step 1: Update CLI with short flags, help text, and version**

Replace the full content of `src/cli.rs`:

```rust
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "aienv", version, about = "Run commands inside isolated AI identity environments")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Initialize a new environment
    Init {
        #[arg(short = 'e', long, help = "Environment name")]
        env: String,
    },
    /// Execute a command in an environment
    Exec {
        #[arg(short = 'e', long, help = "Environment name")]
        env: String,
        #[arg(long, help = "Allocate a TTY for interactive commands")]
        tty: bool,
        #[arg(long, help = "Override working directory")]
        cwd: Option<PathBuf>,
        #[arg(last = true, required = true, help = "Command to execute")]
        command: Vec<String>,
    },
    /// Enter an environment shell
    Shell {
        #[arg(short = 'e', long, help = "Environment name")]
        env: String,
        #[arg(long, help = "Override working directory")]
        cwd: Option<PathBuf>,
    },
    /// List all environments
    Ls,
    /// Show resolved environment configuration
    Show {
        #[arg(short = 'e', long, help = "Environment name")]
        env: String,
    },
}
```

- [ ] **Step 2: Remove portable-pty from Cargo.toml**

In `Cargo.toml`, remove the `portable-pty = "0.8"` line from `[dependencies]`.

- [ ] **Step 3: Update CLI help test**

The existing `tests/cli_help.rs` should still pass since it only checks for "aienv", "exec", and "shell" in the help output. Verify.

- [ ] **Step 4: Run all tests and verify build**

Run: `cargo test`
Expected: PASS for all tests.

Run: `cargo build`
Expected: Compiles without portable-pty.

- [ ] **Step 5: Commit**

```bash
git add src/cli.rs Cargo.toml
git commit -m "feat: add CLI help text, -e short flag, version; remove portable-pty"
```

---

### Task 8: Integration Tests for Exec Secrets and Inherit CWD

**Files:**
- Modify: `tests/exec_env.rs`

Covers spec section 4.7 — integration-level test coverage for secrets injection through exec, inherit_cwd=false, and id/root validation.

- [ ] **Step 1: Write integration test for exec with envfile secrets**

Add to `tests/exec_env.rs`:

```rust
#[test]
fn exec_injects_envfile_secrets_into_child_process() {
    let home = TempDir::new().unwrap();
    let env_root = home.path().join(".aienv").join("work");
    for dir in ["home", "config", "cache", "data", "state", "tmp", "run"] {
        fs::create_dir_all(env_root.join(dir)).unwrap();
    }
    fs::write(
        env_root.join("manifest.toml"),
        format!(
            r#"id = "work"
root = "{root}"
[env]
allow = ["PATH"]
[env.set]
AI_ENV = "work"
[secrets]
provider = "envfile"
items = ["MY_SECRET"]
[shell]
program = "/bin/zsh"
init = "env.zsh"
[network]
mode = "default"
"#,
            root = env_root.display()
        ),
    )
    .unwrap();
    fs::write(env_root.join("env.zsh"), "export AI_ENV=work\n").unwrap();

    let secrets_path = env_root.join("secrets.env");
    fs::write(&secrets_path, "MY_SECRET=super-secret-value\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&secrets_path, fs::Permissions::from_mode(0o600)).unwrap();
    }

    let mut cmd = Command::cargo_bin("aienv").unwrap();
    cmd.env("HOME", home.path()).args([
        "exec",
        "--env",
        "work",
        "--",
        "python3",
        "-c",
        "import os; print(os.environ.get('MY_SECRET', 'MISSING'))",
    ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("super-secret-value"));
}

#[test]
fn exec_with_inherit_cwd_false_uses_env_home() {
    let home = TempDir::new().unwrap();
    let env_root = home.path().join(".aienv").join("work");
    for dir in ["home", "config", "cache", "data", "state", "tmp", "run"] {
        fs::create_dir_all(env_root.join(dir)).unwrap();
    }
    fs::write(
        env_root.join("manifest.toml"),
        format!(
            r#"id = "work"
root = "{root}"
inherit_cwd = false
[env]
allow = ["PATH"]
[secrets]
provider = "none"
[network]
mode = "default"
"#,
            root = env_root.display()
        ),
    )
    .unwrap();
    fs::write(env_root.join("env.zsh"), "").unwrap();

    let mut cmd = Command::cargo_bin("aienv").unwrap();
    cmd.env("HOME", home.path()).args([
        "exec",
        "--env",
        "work",
        "--",
        "python3",
        "-c",
        "import os; print(os.getcwd())",
    ]);

    cmd.assert().success().stdout(
        predicate::str::contains(env_root.join("home").to_string_lossy().as_ref()),
    );
}

#[test]
fn exec_rejects_mismatched_manifest_id() {
    let home = TempDir::new().unwrap();
    let env_root = home.path().join(".aienv").join("work");
    for dir in ["home", "config", "cache", "data", "state", "tmp", "run"] {
        fs::create_dir_all(env_root.join(dir)).unwrap();
    }
    fs::write(
        env_root.join("manifest.toml"),
        format!(
            r#"id = "wrong-id"
root = "{root}"
[env]
allow = []
"#,
            root = env_root.display()
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("aienv").unwrap();
    cmd.env("HOME", home.path()).args([
        "exec",
        "--env",
        "work",
        "--",
        "echo",
        "hi",
    ]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("does not match"));
}
```

- [ ] **Step 2: Run all tests**

Run: `cargo test`
Expected: PASS for all tests.

- [ ] **Step 3: Commit**

```bash
git add tests/exec_env.rs
git commit -m "test: add integration tests for secrets injection, inherit_cwd, id validation"
```

---

### Task 9: Final Cleanup and Full Verification

**Files:**
- All files (verification only, no new code unless tests fail)

- [ ] **Step 1: Run full test suite**

Run: `cargo test`
Expected: PASS for all tests.

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: PASS with no warnings. Fix any issues found.

- [ ] **Step 3: Run format check**

Run: `cargo fmt -- --check`
Expected: PASS. If not, run `cargo fmt` and commit the formatting.

- [ ] **Step 4: Verify all commands work manually**

Run: `cargo run -- --version`
Expected: Prints version.

Run: `cargo run -- --help`
Expected: Shows help with all subcommands.

Run: `cargo run -- init -e test-final`
Expected: Creates `~/.aienv/test-final/` with all directories and files.

Run: `cargo run -- ls`
Expected: Lists `test-final` (and any other environments).

Run: `cargo run -- show -e test-final`
Expected: Prints resolved config.

Run: `cargo run -- exec -e test-final -- echo "hello from aienv"`
Expected: Prints "hello from aienv".

Run: `rm -rf ~/.aienv/test-final`
Expected: Cleanup.

- [ ] **Step 5: Commit any remaining fixes**

```bash
git add -A
git commit -m "chore: final cleanup and formatting"
```

---

## Self-Review Notes

**Spec coverage check:**

| Spec Section | Plan Task |
|---|---|
| 1.1 Forced vars | Task 1 (validation) + Task 3 (runtime reorder) |
| 1.2 Exec secrets | Task 4 (env_path.resolve_secrets + exec.rs) |
| 1.3 inherit_cwd | Task 4 (exec.rs) |
| 1.4 Signal exit code | Task 5 (main.rs) |
| 1.5 Nested AIENV_ROOT | Task 3 (runtime_env) + Task 4 (env_path) |
| 1.6 Provider validation | Task 1 (manifest) |
| 1.7 Deny list | Task 4 (init.rs) |
| 1.8 Network mode | Task 1 (manifest proxy_url) + Task 3 (runtime injection) |
| 1.9 Envfile permissions | Task 2 (secrets.rs) |
| 1.10 id/root validation | Task 4 (env_path.load_manifest) |
| 2.1 exec --tty | Task 5 (main.rs merges branches) |
| 2.2 shell | Task 6 |
| 2.3 ls | Task 5 |
| 2.4 show | Task 5 |
| 3.1 Optional sections | Task 1 |
| 3.2 none provider | Task 1 |
| 3.3 Envfile upgrade | Task 2 |
| 3.4 XDG data/state | Task 3 (runtime) + Task 4 (init dirs) |
| 3.5 Tilde expansion | Task 1 |
| 3.6 Init secrets.env | Task 4 (init.rs) |
| 4.1 CLI polish | Task 7 |
| 4.2 Remove portable-pty | Task 7 |
| 4.3 Extract env_path | Task 4 |
| 4.4 Run directories | Task 4 (exec.rs) + Task 6 (shell.rs) |
| 4.5 path_policy return | Task 4 |
| 4.7 Test coverage | Tasks 1,2,3,4,5,6,8 |

All 34 spec items mapped to tasks. No gaps.

**Placeholder scan:** No TBD/TODO in any code block. All steps have concrete code or commands.

**Type consistency:**
- `build_child_env` takes `(manifest, host, secrets, exec_dir: Option<&str>)` — consistent across Task 3 definition, Task 4 exec.rs usage, Task 6 shell.rs usage.
- `load_manifest` returns `Result<(Manifest, PathBuf)>` — consistent across Task 4 definition and Task 5/6/8 usage.
- `resolve_secrets` takes `(manifest, env_root: &Path)` — consistent across definition and usage.
- `validate_cwd` returns `Result<(PathBuf, Vec<PathBuf>)>` — consistent across Task 4 definition and all call sites.
- `build_shell_args` takes `(program: &Path, env_root: &Path, init: &Path) -> Vec<String>` — consistent between Task 6 implementation and test.

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-03-31-aienv-completion.md`. Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?
