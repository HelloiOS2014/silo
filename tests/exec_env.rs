use aienv::secrets::resolve_from_envfile;
use aienv::{manifest::Manifest, path_policy::validate_cwd, runtime_env::build_child_env};
use assert_cmd::Command;
use predicates::prelude::*;
use std::{collections::BTreeMap, fs, path::PathBuf};
use tempfile::TempDir;

#[test]
fn loads_only_requested_keys_from_envfile() {
    let dir = tempfile::TempDir::new().unwrap();
    let envfile = dir.path().join("secrets.env");
    std::fs::write(
        &envfile,
        "OPENAI_API_KEY=one\nANTHROPIC_API_KEY=two\nUNUSED_KEY=three\n",
    )
    .unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&envfile, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let secrets = resolve_from_envfile(&envfile, &["OPENAI_API_KEY".into()]).unwrap();
    assert_eq!(secrets["OPENAI_API_KEY"], "one");
    assert!(!secrets.contains_key("ANTHROPIC_API_KEY"));
}

#[test]
fn rejects_invalid_envfile_lines() {
    let dir = tempfile::TempDir::new().unwrap();
    let envfile = dir.path().join("secrets.env");
    std::fs::write(&envfile, "OPENAI_API_KEY=one\nBROKEN_LINE\n").unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&envfile, std::fs::Permissions::from_mode(0o600)).unwrap();
    }

    let err = resolve_from_envfile(&envfile, &["OPENAI_API_KEY".into()]).unwrap_err();
    assert!(err.to_string().contains("invalid envfile line"));
}

#[test]
fn rejects_missing_requested_keys_from_envfile() {
    let dir = tempfile::TempDir::new().unwrap();
    let envfile = dir.path().join("secrets.env");
    std::fs::write(&envfile, "OPENAI_API_KEY=one\n").unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&envfile, std::fs::Permissions::from_mode(0o600)).unwrap();
    }

    let err = resolve_from_envfile(&envfile, &["ANTHROPIC_API_KEY".into()]).unwrap_err();
    assert!(err.to_string().contains("missing secret"));
}

#[test]
fn builds_sanitized_child_env() {
    let manifest = Manifest::parse(
        r#"
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
"#,
    )
    .unwrap();

    let mut host = BTreeMap::new();
    host.insert("TERM".into(), "xterm-256color".into());
    host.insert("PATH".into(), "/usr/bin".into());
    host.insert("OPENAI_API_KEY".into(), "should-not-leak".into());

    let env = build_child_env(&manifest, &host, BTreeMap::new());
    assert_eq!(env["AI_ENV"], "work");
    assert_eq!(env["HOME"], "/tmp/work/home");
    assert_eq!(env["XDG_CONFIG_HOME"], "/tmp/work/config");
    assert_eq!(env["XDG_CACHE_HOME"], "/tmp/work/cache");
    assert_eq!(env["TMPDIR"], "/tmp/work/tmp");
    assert_eq!(env["TERM"], "xterm-256color");
    assert_eq!(env["PATH"], "/usr/bin");
    assert!(!env.contains_key("OPENAI_API_KEY"));
    assert!(!env.contains_key("SSH_AUTH_SOCK"));
}

#[test]
fn rejects_missing_cwd() {
    let cwd = PathBuf::from("/definitely/missing/path");
    let err = validate_cwd(&cwd, &[]).unwrap_err();
    assert!(err.to_string().contains("cwd"));
}

#[test]
fn allows_normal_cwd_even_when_shared_paths_are_present() {
    let temp = tempfile::tempdir().unwrap();
    let cwd = temp.path().join("cwd");
    std::fs::create_dir_all(&cwd).unwrap();

    let shared = tempfile::tempdir().unwrap();
    let validated = validate_cwd(&cwd, &[shared.path().to_path_buf()]).unwrap();
    assert_eq!(validated, std::fs::canonicalize(&cwd).unwrap());
}

#[test]
fn rejects_invalid_shared_paths() {
    let temp = tempfile::tempdir().unwrap();
    let cwd = temp.path().join("cwd");
    std::fs::create_dir_all(&cwd).unwrap();

    let missing_shared = temp.path().join("missing-shared");
    let err = validate_cwd(&cwd, &[missing_shared]).unwrap_err();
    assert!(err.to_string().contains("shared path"));
}

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
    cmd.env("HOME", home.path()).args([
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
        .stdout(predicate::str::contains(
            env_root.join("home").display().to_string(),
        ))
        .stdout(predicate::str::contains("work"));
}

#[test]
fn envfile_supports_comments_and_blank_lines() {
    let dir = tempfile::TempDir::new().unwrap();
    let envfile = dir.path().join("secrets.env");
    std::fs::write(
        &envfile,
        "# this is a comment\n\nOPENAI_API_KEY=one\n# another comment\nGEMINI_KEY=two\n",
    )
    .unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&envfile, std::fs::Permissions::from_mode(0o600)).unwrap();
    }

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

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&envfile, std::fs::Permissions::from_mode(0o600)).unwrap();
    }

    let secrets = resolve_from_envfile(&envfile, &["OPENAI_API_KEY".into()]).unwrap();
    assert_eq!(secrets["OPENAI_API_KEY"], "one");
}

#[test]
fn envfile_supports_double_quoted_values() {
    let dir = tempfile::TempDir::new().unwrap();
    let envfile = dir.path().join("secrets.env");
    std::fs::write(&envfile, "KEY=\"hello world\"\nKEY2=\"line\\nbreak\"\n").unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&envfile, std::fs::Permissions::from_mode(0o600)).unwrap();
    }

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

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&envfile, std::fs::Permissions::from_mode(0o600)).unwrap();
    }

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

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&envfile, std::fs::Permissions::from_mode(0o600)).unwrap();
    }

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
