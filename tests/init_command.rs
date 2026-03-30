use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

use aienv::manifest::Manifest;

#[test]
fn init_creates_environment_layout() {
    let home = TempDir::new().unwrap();
    let env_root = home.path().join(".aienv").join("work");

    let mut cmd = Command::cargo_bin("aienv").unwrap();
    cmd.env("HOME", home.path()).args(["init", "--env", "work"]);

    cmd.assert().success();
    assert!(env_root.join("manifest.toml").exists());
    assert!(env_root.join("home").exists());
    assert!(env_root.join("config").exists());
    assert!(env_root.join("cache").exists());
    assert!(env_root.join("tmp").exists());
    assert!(env_root.join("run").exists());
    assert!(env_root.join("env.zsh").exists());

    let raw_manifest = fs::read_to_string(env_root.join("manifest.toml")).unwrap();
    let manifest = Manifest::parse(&raw_manifest).unwrap();
    assert_eq!(manifest.id, "work");
    assert!(manifest.inherit_cwd);
    assert_eq!(manifest.shell.init.to_string_lossy(), "env.zsh");
}

#[test]
fn init_is_idempotent() {
    let home = TempDir::new().unwrap();
    let env_root = home.path().join(".aienv").join("work");

    let mut first = Command::cargo_bin("aienv").unwrap();
    first
        .env("HOME", home.path())
        .args(["init", "--env", "work"])
        .assert()
        .success();

    fs::write(env_root.join("manifest.toml"), "id = \"custom\"\n").unwrap();
    fs::write(env_root.join("env.zsh"), "export AI_ENV=custom\n").unwrap();

    let mut second = Command::cargo_bin("aienv").unwrap();
    second
        .env("HOME", home.path())
        .args(["init", "--env", "work"])
        .assert()
        .success()
        .stdout(predicate::str::contains("already initialized"));

    assert_eq!(
        fs::read_to_string(env_root.join("manifest.toml")).unwrap(),
        "id = \"custom\"\n"
    );
    assert_eq!(
        fs::read_to_string(env_root.join("env.zsh")).unwrap(),
        "export AI_ENV=custom\n"
    );
}

#[test]
fn init_rejects_path_escaping_env_names() {
    let home = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("aienv").unwrap();
    cmd.env("HOME", home.path())
        .args(["init", "--env", "../escape"]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("environment name"));
}
