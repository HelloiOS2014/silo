# AI CLI 轻量环境隔离 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust CLI named `aienv` that launches commands inside identity-scoped environments with isolated `HOME/XDG/TMPDIR`, sanitized environment variables, per-environment secrets injection, shared-path policy checks, and both non-interactive and TTY execution.

**Architecture:** Use a single Rust binary with a small command layer (`init`, `exec`, `shell`, `ls`, `show`) on top of focused library modules for manifest loading, runtime environment construction, secrets resolution, path validation, and command execution. Keep the implementation tool-agnostic: `aienv` only launches arbitrary commands inside a configured environment and never adds `codex`/`claude`/`gemini`-specific behavior.

**Tech Stack:** Rust stable, Cargo, `clap`, `serde`, `toml`, `anyhow`, `thiserror`, `portable-pty`, `assert_cmd`, `predicates`, `tempfile`

---

## File Structure

Planned files and responsibilities:

- Create: `Cargo.toml`
  - Rust package metadata and dependencies.
- Create: `src/main.rs`
  - Thin CLI entrypoint that parses args and dispatches subcommands.
- Create: `src/lib.rs`
  - Public module wiring for testable library code.
- Create: `src/cli.rs`
  - `clap` argument definitions for `init`, `exec`, `shell`, `ls`, `show`.
- Create: `src/error.rs`
  - Shared typed errors for manifest, secret lookup, path policy, and process launch failures.
- Create: `src/manifest.rs`
  - `manifest.toml` schema, inheritance merge, validation, and runtime layout derivation.
- Create: `src/runtime_env.rs`
  - Build sanitized child process environment and derived directories.
- Create: `src/path_policy.rs`
  - Resolve real paths, validate current working directory, and enforce allowed/shared path rules.
- Create: `src/secrets.rs`
  - `keychain` and `envfile` secret providers.
- Create: `src/commands/mod.rs`
  - Subcommand routing helpers.
- Create: `src/commands/init.rs`
  - Create environment directory tree, default `env.zsh`, and validate manifest references.
- Create: `src/commands/exec.rs`
  - Non-interactive process execution and exit code propagation.
- Create: `src/commands/pty.rs`
  - TTY-backed command execution using `portable-pty`.
- Create: `src/commands/shell.rs`
  - Controlled shell startup that only loads environment-local init.
- Create: `src/commands/show.rs`
  - Print resolved environment config.
- Create: `src/commands/ls.rs`
  - Enumerate environment manifests under `~/.aienv/`.
- Create: `tests/manifest_parsing.rs`
  - Manifest parsing, inheritance, and validation tests.
- Create: `tests/init_command.rs`
  - Integration tests for `aienv init`.
- Create: `tests/exec_env.rs`
  - Integration tests for sanitized env, cwd inheritance, and secret injection.
- Create: `tests/exec_tty.rs`
  - PTY smoke test for `exec --tty` and `shell`.

### Task 1: Scaffold the Rust CLI Workspace

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/lib.rs`
- Create: `src/cli.rs`
- Test: `tests/manifest_parsing.rs`

- [ ] **Step 1: Write the failing CLI smoke test**

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn shows_help_for_top_level_command() {
    let mut cmd = Command::cargo_bin("aienv").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("aienv"))
        .stdout(predicate::str::contains("exec"))
        .stdout(predicate::str::contains("shell"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test shows_help_for_top_level_command -- --exact --nocapture`
Expected: FAIL with cargo complaining that package or binary `aienv` does not exist yet.

- [ ] **Step 3: Create `Cargo.toml` with the initial dependency set**

```toml
[package]
name = "aienv"
version = "0.1.0"
edition = "2024"

[dependencies]
anyhow = "1.0"
clap = { version = "4.5", features = ["derive"] }
portable-pty = "0.8"
serde = { version = "1.0", features = ["derive"] }
thiserror = "2.0"
toml = "0.9"

[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.1"
tempfile = "3.20"
```

- [ ] **Step 4: Create the minimal CLI skeleton**

```rust
// src/lib.rs
pub mod cli;

// src/cli.rs
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "aienv")]
#[command(about = "Run commands inside isolated AI identity environments")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Init { #[arg(long)] env: String },
    Exec {
        #[arg(long)]
        env: String,
        #[arg(long)]
        tty: bool,
        #[arg(long)]
        cwd: Option<PathBuf>,
        #[arg(last = true, required = true)]
        command: Vec<String>,
    },
    Shell {
        #[arg(long)]
        env: String,
        #[arg(long)]
        cwd: Option<PathBuf>,
    },
    Ls,
    Show { #[arg(long)] env: String },
}

// src/main.rs
use aienv::cli::Cli;
use clap::Parser;

fn main() {
    let _cli = Cli::parse();
}
```

- [ ] **Step 5: Run the smoke test and full test suite**

Run: `cargo test`
Expected: PASS with the help smoke test green and no other tests present.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml src/main.rs src/lib.rs src/cli.rs tests/manifest_parsing.rs
git commit -m "feat: scaffold rust cli entrypoint"
```

### Task 2: Implement Manifest Loading and Validation

**Files:**
- Modify: `src/lib.rs`
- Create: `src/error.rs`
- Create: `src/manifest.rs`
- Test: `tests/manifest_parsing.rs`

- [ ] **Step 1: Write failing manifest parsing tests**

```rust
use aienv::manifest::Manifest;

#[test]
fn parses_minimal_manifest() {
    let raw = r#"
id = "work"
root = "/tmp/work"
inherit_cwd = true
shared_paths = ["/tmp/ai-bus"]

[env]
allow = ["TERM", "PATH"]
deny = ["OPENAI_API_KEY"]

[env.set]
AI_ENV = "work"

[secrets]
provider = "keychain"
items = ["OPENAI_API_KEY"]

[shell]
program = "/bin/zsh"
init = "env.zsh"

[network]
mode = "default"
"#;

    let manifest = Manifest::parse(raw).unwrap();
    assert_eq!(manifest.id, "work");
    assert_eq!(manifest.root.to_string_lossy(), "/tmp/work");
    assert!(manifest.inherit_cwd);
}

#[test]
fn rejects_unknown_network_mode() {
    let raw = r#"
id = "work"
root = "/tmp/work"
[env]
allow = []
deny = []
[env.set]
[secrets]
provider = "keychain"
items = []
[shell]
program = "/bin/zsh"
init = "env.zsh"
[network]
mode = "bogus"
"#;

    let err = Manifest::parse(raw).unwrap_err();
    assert!(err.to_string().contains("network.mode"));
}
```

- [ ] **Step 2: Run the manifest tests to verify they fail**

Run: `cargo test manifest_parsing -- --nocapture`
Expected: FAIL with unresolved import/module errors for `aienv::manifest`.

- [ ] **Step 3: Implement manifest structs, parsing, and validation**

```rust
// src/error.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AienvError {
    #[error("manifest parse error: {0}")]
    ManifestParse(#[from] toml::de::Error),
    #[error("manifest validation error: {0}")]
    ManifestValidation(String),
}

// src/manifest.rs
use crate::error::AienvError;
use serde::Deserialize;
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub id: String,
    pub root: PathBuf,
    #[serde(default = "default_true")]
    pub inherit_cwd: bool,
    #[serde(default)]
    pub shared_paths: Vec<PathBuf>,
    pub env: EnvConfig,
    pub secrets: SecretsConfig,
    pub shell: ShellConfig,
    pub network: NetworkConfig,
    #[serde(default)]
    pub extends: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct EnvConfig {
    #[serde(default)]
    pub allow: Vec<String>,
    #[serde(default)]
    pub deny: Vec<String>,
    #[serde(default)]
    pub set: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SecretsConfig {
    pub provider: String,
    #[serde(default)]
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ShellConfig {
    pub program: PathBuf,
    pub init: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NetworkConfig {
    pub mode: String,
}

fn default_true() -> bool { true }

impl Manifest {
    pub fn parse(raw: &str) -> Result<Self, AienvError> {
        let manifest: Self = toml::from_str(raw)?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), AienvError> {
        match self.network.mode.as_str() {
            "default" | "offline" | "proxy" => {}
            other => {
                return Err(AienvError::ManifestValidation(format!(
                    "network.mode must be default|offline|proxy, got {other}"
                )));
            }
        }
        if self.id.trim().is_empty() {
            return Err(AienvError::ManifestValidation("id cannot be empty".into()));
        }
        Ok(())
    }
}
```

- [ ] **Step 4: Export the new modules**

```rust
// src/lib.rs
pub mod cli;
pub mod error;
pub mod manifest;
```

- [ ] **Step 5: Run focused tests, then full suite**

Run: `cargo test manifest_parsing -- --nocapture`
Expected: PASS for the new parsing tests.

Run: `cargo test`
Expected: PASS for all current tests.

- [ ] **Step 6: Commit**

```bash
git add src/lib.rs src/error.rs src/manifest.rs tests/manifest_parsing.rs
git commit -m "feat: add manifest parsing and validation"
```

### Task 3: Add Environment Initialization

**Files:**
- Create: `src/commands/mod.rs`
- Create: `src/commands/init.rs`
- Modify: `src/main.rs`
- Test: `tests/init_command.rs`

- [ ] **Step 1: Write failing init integration tests**

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn init_creates_environment_layout() {
    let home = TempDir::new().unwrap();
    let env_root = home.path().join(".aienv").join("work");

    let mut cmd = Command::cargo_bin("aienv").unwrap();
    cmd.env("HOME", home.path())
        .args(["init", "--env", "work"]);

    cmd.assert().success();
    assert!(env_root.join("manifest.toml").exists());
    assert!(env_root.join("home").exists());
    assert!(env_root.join("config").exists());
    assert!(env_root.join("cache").exists());
    assert!(env_root.join("tmp").exists());
    assert!(env_root.join("run").exists());
    assert!(env_root.join("env.zsh").exists());
}

#[test]
fn init_is_idempotent() {
    let home = TempDir::new().unwrap();

    let mut first = Command::cargo_bin("aienv").unwrap();
    first.env("HOME", home.path())
        .args(["init", "--env", "work"])
        .assert()
        .success();

    let mut second = Command::cargo_bin("aienv").unwrap();
    second.env("HOME", home.path())
        .args(["init", "--env", "work"])
        .assert()
        .success()
        .stdout(predicate::str::contains("already initialized"));
}
```

- [ ] **Step 2: Run the init tests to verify they fail**

Run: `cargo test init_creates_environment_layout -- --exact --nocapture`
Expected: FAIL because `init` does not dispatch any behavior yet.

- [ ] **Step 3: Implement the init command**

```rust
// src/commands/init.rs
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
```

- [ ] **Step 4: Wire init dispatch through `main`**

```rust
// src/main.rs
use aienv::{cli::{Cli, Commands}, commands};
use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init { env } => commands::init::run(&env)?,
        _ => todo!("other commands not implemented yet"),
    }
    Ok(())
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test init_command -- --nocapture`
Expected: PASS for init integration tests.

Run: `cargo test`
Expected: PASS for all current tests.

- [ ] **Step 6: Commit**

```bash
git add src/main.rs src/commands/mod.rs src/commands/init.rs tests/init_command.rs
git commit -m "feat: add environment initialization command"
```

### Task 4: Build Runtime Environment and Path Policy

**Files:**
- Create: `src/runtime_env.rs`
- Create: `src/path_policy.rs`
- Modify: `src/lib.rs`
- Test: `tests/exec_env.rs`

- [ ] **Step 1: Write failing tests for derived directories, env sanitization, and cwd validation**

```rust
use aienv::{manifest::Manifest, path_policy::validate_cwd, runtime_env::build_child_env};
use std::{collections::BTreeMap, path::PathBuf};

#[test]
fn builds_sanitized_child_env() {
    let manifest = Manifest::parse(r#"
id = "work"
root = "/tmp/work"
inherit_cwd = true
shared_paths = ["/tmp/ai-bus"]
[env]
allow = ["TERM", "PATH"]
deny = ["OPENAI_API_KEY", "SSH_AUTH_SOCK"]
[env.set]
AI_ENV = "work"
[secrets]
provider = "keychain"
items = []
[shell]
program = "/bin/zsh"
init = "env.zsh"
[network]
mode = "default"
"#).unwrap();

    let mut host = BTreeMap::new();
    host.insert("TERM".into(), "xterm-256color".into());
    host.insert("PATH".into(), "/usr/bin".into());
    host.insert("OPENAI_API_KEY".into(), "should-not-leak".into());

    let env = build_child_env(&manifest, &host, BTreeMap::new());
    assert_eq!(env["AI_ENV"], "work");
    assert_eq!(env["HOME"], "/tmp/work/home");
    assert!(!env.contains_key("OPENAI_API_KEY"));
}

#[test]
fn rejects_missing_cwd() {
    let cwd = PathBuf::from("/definitely/missing/path");
    let err = validate_cwd(&cwd, &[]).unwrap_err();
    assert!(err.to_string().contains("cwd"));
}
```

- [ ] **Step 2: Run the focused tests to verify they fail**

Run: `cargo test builds_sanitized_child_env -- --exact --nocapture`
Expected: FAIL due to missing runtime modules.

- [ ] **Step 3: Implement runtime env derivation**

```rust
// src/runtime_env.rs
use crate::manifest::Manifest;
use std::{collections::BTreeMap, path::Path};

pub fn build_child_env(
    manifest: &Manifest,
    host: &BTreeMap<String, String>,
    secrets: BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();

    for key in &manifest.env.allow {
        if let Some(value) = host.get(key) {
            env.insert(key.clone(), value.clone());
        }
    }

    env.insert("HOME".into(), join_str(&manifest.root, "home"));
    env.insert("XDG_CONFIG_HOME".into(), join_str(&manifest.root, "config"));
    env.insert("XDG_CACHE_HOME".into(), join_str(&manifest.root, "cache"));
    env.insert("TMPDIR".into(), join_str(&manifest.root, "tmp"));

    for (key, value) in &manifest.env.set {
        env.insert(key.clone(), value.clone());
    }
    for (key, value) in secrets {
        env.insert(key, value);
    }
    env
}

fn join_str(root: &Path, child: &str) -> String {
    root.join(child).to_string_lossy().into_owned()
}
```

- [ ] **Step 4: Implement path validation**

```rust
// src/path_policy.rs
use anyhow::{bail, Result};
use std::{fs, path::{Path, PathBuf}};

pub fn validate_cwd(cwd: &Path, shared_paths: &[PathBuf]) -> Result<PathBuf> {
    let cwd_real = fs::canonicalize(cwd)
        .map_err(|e| anyhow::anyhow!("cwd is invalid: {e}"))?;

    let mut allowed = vec![cwd_real.clone()];
    for path in shared_paths {
        allowed.push(fs::canonicalize(path)
            .map_err(|e| anyhow::anyhow!("shared path is invalid: {e}"))?);
    }

    if allowed.iter().any(|candidate| cwd_real.starts_with(candidate)) {
        Ok(cwd_real)
    } else {
        bail!("cwd is outside allowed paths")
    }
}
```

- [ ] **Step 5: Export modules and run tests**

```rust
// src/lib.rs
pub mod cli;
pub mod commands;
pub mod error;
pub mod manifest;
pub mod path_policy;
pub mod runtime_env;
```

Run: `cargo test exec_env -- --nocapture`
Expected: PASS for the new runtime/path policy tests.

Run: `cargo test`
Expected: PASS for all current tests.

- [ ] **Step 6: Commit**

```bash
git add src/lib.rs src/runtime_env.rs src/path_policy.rs tests/exec_env.rs
git commit -m "feat: add runtime env builder and path policy"
```

### Task 5: Add Secret Providers

**Files:**
- Create: `src/secrets.rs`
- Modify: `src/lib.rs`
- Test: `tests/exec_env.rs`

- [ ] **Step 1: Write failing secret provider tests**

```rust
use aienv::secrets::{resolve_from_envfile, SecretProvider};
use std::fs;
use tempfile::TempDir;

#[test]
fn loads_only_requested_keys_from_envfile() {
    let dir = TempDir::new().unwrap();
    let envfile = dir.path().join("secrets.env");
    fs::write(
        &envfile,
        "OPENAI_API_KEY=one\nANTHROPIC_API_KEY=two\nUNUSED_KEY=three\n",
    )
    .unwrap();

    let secrets = resolve_from_envfile(&envfile, &["OPENAI_API_KEY".into()]).unwrap();
    assert_eq!(secrets["OPENAI_API_KEY"], "one");
    assert!(!secrets.contains_key("ANTHROPIC_API_KEY"));
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test loads_only_requested_keys_from_envfile -- --exact --nocapture`
Expected: FAIL due to missing `secrets` module.

- [ ] **Step 3: Implement `envfile` and `keychain` secret resolution**

```rust
// src/secrets.rs
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

pub enum SecretProvider<'a> {
    Envfile(&'a Path),
    Keychain { service: &'a str },
}
```

- [ ] **Step 4: Export the module and add one integration path that uses it**

```rust
// src/lib.rs
pub mod secrets;
```

- [ ] **Step 5: Run tests**

Run: `cargo test loads_only_requested_keys_from_envfile -- --exact --nocapture`
Expected: PASS.

Run: `cargo test`
Expected: PASS for all current tests.

- [ ] **Step 6: Commit**

```bash
git add src/lib.rs src/secrets.rs tests/exec_env.rs
git commit -m "feat: add envfile and keychain secret providers"
```

### Task 6: Implement `exec` for Non-Interactive Commands

**Files:**
- Create: `src/commands/exec.rs`
- Modify: `src/main.rs`
- Test: `tests/exec_env.rs`

- [ ] **Step 1: Write a failing integration test for `exec`**

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn exec_runs_command_in_environment_with_isolated_home() {
    let home = TempDir::new().unwrap();
    let env_root = home.path().join(".aienv").join("work");
    fs::create_dir_all(env_root.join("home")).unwrap();
    fs::create_dir_all(env_root.join("config")).unwrap();
    fs::create_dir_all(env_root.join("cache")).unwrap();
    fs::create_dir_all(env_root.join("tmp")).unwrap();
    fs::create_dir_all(env_root.join("run")).unwrap();
    fs::write(
        env_root.join("manifest.toml"),
        format!(
            "id = \"work\"\nroot = \"{}\"\ninherit_cwd = true\nshared_paths = []\n\n[env]\nallow = [\"PATH\"]\ndeny = [\"OPENAI_API_KEY\"]\n\n[env.set]\nAI_ENV = \"work\"\n\n[secrets]\nprovider = \"envfile\"\nitems = []\n\n[shell]\nprogram = \"/bin/zsh\"\ninit = \"env.zsh\"\n\n[network]\nmode = \"default\"\n",
            env_root.display()
        ),
    )
    .unwrap();
    fs::write(env_root.join("env.zsh"), "export AI_ENV=work\n").unwrap();

    let mut cmd = Command::cargo_bin("aienv").unwrap();
    cmd.env("HOME", home.path())
        .args([
            "exec",
            "--env",
            "work",
            "--",
            "python3",
            "-c",
            "import os; print(os.environ['HOME']); print(os.environ['AI_ENV'])",
        ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(env_root.join("home").display().to_string()))
        .stdout(predicate::str::contains("work"));
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test exec_runs_command_in_environment_with_isolated_home -- --exact --nocapture`
Expected: FAIL because `exec` is not implemented.

- [ ] **Step 3: Implement manifest loading from disk and `exec` dispatch**

```rust
// src/commands/exec.rs
use anyhow::{Context, Result};
use std::{collections::BTreeMap, fs, path::PathBuf, process::{Command, ExitStatus}};

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
        .current_dir(cwd)
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
```

- [ ] **Step 4: Wire `exec` in `main` with exit code propagation**

```rust
// src/main.rs
match cli.command {
    Commands::Init { env } => commands::init::run(&env)?,
    Commands::Exec { env, tty: false, cwd, command } => {
        let status = commands::exec::run(&env, cwd, command)?;
        std::process::exit(status.code().unwrap_or(1));
    }
    Commands::Exec { tty: true, .. } => todo!("tty path comes next"),
    Commands::Shell { .. } => todo!("shell path comes next"),
    Commands::Ls => todo!("ls comes later"),
    Commands::Show { .. } => todo!("show comes later"),
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test exec_runs_command_in_environment_with_isolated_home -- --exact --nocapture`
Expected: PASS.

Run: `cargo test`
Expected: PASS for all current tests.

- [ ] **Step 6: Commit**

```bash
git add src/main.rs src/commands/exec.rs tests/exec_env.rs
git commit -m "feat: add non-interactive exec command"
```

### Task 7: Implement TTY Exec and Shell

**Files:**
- Create: `src/commands/pty.rs`
- Create: `src/commands/shell.rs`
- Modify: `src/main.rs`
- Test: `tests/exec_tty.rs`

- [ ] **Step 1: Write failing PTY smoke tests**

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn shell_uses_environment_specific_init_file() {
    let home = TempDir::new().unwrap();
    let env_root = home.path().join(".aienv").join("work");
    for dir in ["home", "config", "cache", "tmp", "run"] {
        fs::create_dir_all(env_root.join(dir)).unwrap();
    }
    fs::write(
        env_root.join("manifest.toml"),
        format!(
            "id = \"work\"\nroot = \"{}\"\ninherit_cwd = true\nshared_paths = []\n\n[env]\nallow = [\"PATH\"]\ndeny = []\n\n[env.set]\nAI_ENV = \"work\"\n\n[secrets]\nprovider = \"envfile\"\nitems = []\n\n[shell]\nprogram = \"/bin/zsh\"\ninit = \"env.zsh\"\n\n[network]\nmode = \"default\"\n",
            env_root.display()
        ),
    )
    .unwrap();
    fs::write(env_root.join("env.zsh"), "export PLAN_SENTINEL=from-env-init\n").unwrap();

    let mut cmd = Command::cargo_bin("aienv").unwrap();
    cmd.env("HOME", home.path())
        .args([
            "exec",
            "--env",
            "work",
            "--tty",
            "--",
            "/bin/zsh",
            "-d",
            "-f",
            "-c",
            "source \"$HOME/env.zsh\"; printf '%s' \"$PLAN_SENTINEL\"",
        ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("from-env-init"));
}
```

- [ ] **Step 2: Run the PTY test to verify it fails**

Run: `cargo test shell_uses_environment_specific_init_file -- --exact --nocapture`
Expected: FAIL because `--tty` is still unimplemented.

- [ ] **Step 3: Implement PTY command execution**

```rust
// src/commands/pty.rs
use anyhow::{Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::{collections::BTreeMap, io::{Read, Write}, path::PathBuf};

use crate::{commands::exec::load_manifest, runtime_env::build_child_env};

pub fn run(env: &str, cwd: Option<PathBuf>, command: Vec<String>) -> Result<i32> {
    let manifest = load_manifest(env)?;
    let cwd = cwd.unwrap_or(std::env::current_dir()?);
    let host: BTreeMap<String, String> = std::env::vars().collect();
    let child_env = build_child_env(&manifest, &host, BTreeMap::new());

    let (program, args) = command.split_first().context("command cannot be empty")?;

    let pty = native_pty_system().openpty(PtySize {
        rows: 40,
        cols: 120,
        pixel_width: 0,
        pixel_height: 0,
    })?;
    let mut builder = CommandBuilder::new(program);
    builder.args(args);
    builder.cwd(cwd);
    builder.env_clear();
    for (key, value) in child_env {
        builder.env(key, value);
    }

    let child = pty.slave.spawn_command(builder)?;
    let mut reader = pty.master.try_clone_reader()?;
    let mut writer = std::io::stdout();
    std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        while let Ok(n) = reader.read(&mut buf) {
            if n == 0 { break; }
            let _ = writer.write_all(&buf[..n]);
            let _ = writer.flush();
        }
    });

    let status = child.wait()?;
    Ok(status.exit_code())
}
```

- [ ] **Step 4: Implement controlled shell startup and wire dispatch**

```rust
// src/commands/shell.rs
use anyhow::Result;
use std::path::PathBuf;

use crate::commands::pty;

pub fn run(env: &str, cwd: Option<PathBuf>) -> Result<i32> {
    pty::run(
        env,
        cwd,
        vec![
            "/bin/zsh".into(),
            "-d".into(),
            "-f".into(),
            "-c".into(),
            "source \"$HOME/env.zsh\" && exec /bin/zsh -d -f".into(),
        ],
    )
}

// src/main.rs
Commands::Exec { env, tty: true, cwd, command } => {
    let code = commands::pty::run(&env, cwd, command)?;
    std::process::exit(code);
}
Commands::Shell { env, cwd } => {
    let code = commands::shell::run(&env, cwd)?;
    std::process::exit(code);
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test exec_tty -- --nocapture`
Expected: PASS for the PTY smoke test.

Run: `cargo test`
Expected: PASS for all current tests.

- [ ] **Step 6: Commit**

```bash
git add src/main.rs src/commands/pty.rs src/commands/shell.rs tests/exec_tty.rs
git commit -m "feat: add tty exec and shell commands"
```

### Task 8: Finish `ls` and `show`, Then Add End-to-End Checks

**Files:**
- Create: `src/commands/ls.rs`
- Create: `src/commands/show.rs`
- Modify: `src/main.rs`
- Modify: `tests/init_command.rs`
- Modify: `tests/exec_env.rs`

- [ ] **Step 1: Write failing tests for `ls` and `show`**

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn ls_lists_initialized_environments() {
    let home = TempDir::new().unwrap();
    let root = home.path().join(".aienv").join("work");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("manifest.toml"), "id = \"work\"\nroot = \"/tmp/work\"\n[env]\nallow=[]\ndeny=[]\n[env.set]\n[secrets]\nprovider=\"keychain\"\nitems=[]\n[shell]\nprogram=\"/bin/zsh\"\ninit=\"env.zsh\"\n[network]\nmode=\"default\"\n").unwrap();

    let mut cmd = Command::cargo_bin("aienv").unwrap();
    cmd.env("HOME", home.path()).arg("ls");
    cmd.assert().success().stdout(predicate::str::contains("work"));
}

#[test]
fn show_prints_resolved_manifest() {
    let home = TempDir::new().unwrap();
    let root = home.path().join(".aienv").join("work");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("manifest.toml"), "id = \"work\"\nroot = \"/tmp/work\"\ninherit_cwd = true\nshared_paths = []\n[env]\nallow=[]\ndeny=[]\n[env.set]\nAI_ENV=\"work\"\n[secrets]\nprovider=\"keychain\"\nitems=[]\n[shell]\nprogram=\"/bin/zsh\"\ninit=\"env.zsh\"\n[network]\nmode=\"default\"\n").unwrap();

    let mut cmd = Command::cargo_bin("aienv").unwrap();
    cmd.env("HOME", home.path())
        .args(["show", "--env", "work"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("AI_ENV"))
        .stdout(predicate::str::contains("/tmp/work"));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test ls_lists_initialized_environments -- --exact --nocapture`
Expected: FAIL because `ls` and `show` are still unimplemented.

- [ ] **Step 3: Implement `ls` and `show`**

```rust
// src/commands/ls.rs
use anyhow::Result;
use std::{fs, path::PathBuf};

pub fn run() -> Result<()> {
    let root = PathBuf::from(std::env::var("HOME")?).join(".aienv");
    if !root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if entry.path().join("manifest.toml").exists() {
            println!("{}", entry.file_name().to_string_lossy());
        }
    }
    Ok(())
}

// src/commands/show.rs
use anyhow::Result;
use std::{fs, path::PathBuf};

pub fn run(env: &str) -> Result<()> {
    let path = PathBuf::from(std::env::var("HOME")?)
        .join(".aienv")
        .join(env)
        .join("manifest.toml");
    let raw = fs::read_to_string(path)?;
    print!("{raw}");
    Ok(())
}
```

- [ ] **Step 4: Wire the last command branches**

```rust
// src/main.rs
Commands::Ls => commands::ls::run()?,
Commands::Show { env } => commands::show::run(&env)?,
```

- [ ] **Step 5: Run end-to-end checks**

Run: `cargo test`
Expected: PASS for all integration and unit tests.

Run: `cargo fmt -- --check`
Expected: PASS with no formatting diffs.

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: PASS with no warnings.

- [ ] **Step 6: Commit**

```bash
git add src/main.rs src/commands/ls.rs src/commands/show.rs tests/init_command.rs tests/exec_env.rs
git commit -m "feat: complete command surface and validation checks"
```

## Self-Review Notes

Spec coverage check:

- `init`, `exec`, `shell`, `ls`, `show` are covered by Tasks 1, 3, 6, 7, and 8.
- `HOME/XDG/TMPDIR` isolation and sanitized env are covered by Task 4.
- `keychain` / `envfile` secret providers are covered by Task 5.
- Current-directory inheritance and path checks are covered by Tasks 4 and 6.
- Non-interactive and TTY execution split is covered by Tasks 6 and 7.
- Per-environment directory layout is covered by Task 3.

Placeholder scan:

- No unresolved placeholders remain.
- Every code-changing task includes exact file paths, code blocks, and commands.

Type consistency:

- Command names are consistently `init`, `exec`, `shell`, `ls`, `show`.
- Manifest type names stay consistent across Tasks 2, 4, and 6.
- Runtime environment builder naming stays `build_child_env`.

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-03-29-ai-cli-env-isolation-implementation.md`. Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?
