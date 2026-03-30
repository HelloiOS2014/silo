use aienv::secrets::{resolve_from_envfile, SecretProvider};
use aienv::{manifest::Manifest, path_policy::validate_cwd, runtime_env::build_child_env};
use std::{collections::BTreeMap, path::PathBuf};

#[test]
fn loads_only_requested_keys_from_envfile() {
    let dir = tempfile::TempDir::new().unwrap();
    let envfile = dir.path().join("secrets.env");
    std::fs::write(
        &envfile,
        "OPENAI_API_KEY=one\nANTHROPIC_API_KEY=two\nUNUSED_KEY=three\n",
    )
    .unwrap();

    let _provider = SecretProvider::Envfile(&envfile);
    let _unused_provider = SecretProvider::Keychain { service: "aienv.test" };
    let secrets = resolve_from_envfile(&envfile, &["OPENAI_API_KEY".into()]).unwrap();
    assert_eq!(secrets["OPENAI_API_KEY"], "one");
    assert!(!secrets.contains_key("ANTHROPIC_API_KEY"));
}

#[test]
fn rejects_invalid_envfile_lines() {
    let dir = tempfile::TempDir::new().unwrap();
    let envfile = dir.path().join("secrets.env");
    std::fs::write(&envfile, "OPENAI_API_KEY=one\nBROKEN_LINE\n").unwrap();

    let err = resolve_from_envfile(&envfile, &["OPENAI_API_KEY".into()]).unwrap_err();
    assert!(err.to_string().contains("invalid envfile line"));
}

#[test]
fn rejects_missing_requested_keys_from_envfile() {
    let dir = tempfile::TempDir::new().unwrap();
    let envfile = dir.path().join("secrets.env");
    std::fs::write(&envfile, "OPENAI_API_KEY=one\n").unwrap();

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
