use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

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
}

#[test]
fn init_is_idempotent() {
    let home = TempDir::new().unwrap();

    let mut first = Command::cargo_bin("aienv").unwrap();
    first
        .env("HOME", home.path())
        .args(["init", "--env", "work"])
        .assert()
        .success();

    let mut second = Command::cargo_bin("aienv").unwrap();
    second
        .env("HOME", home.path())
        .args(["init", "--env", "work"])
        .assert()
        .success()
        .stdout(predicate::str::contains("already initialized"));
}
