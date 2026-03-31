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
fn shell_command_builds_correct_zsh_args() {
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

#[test]
fn shell_command_builds_generic_args_for_unknown_shell() {
    use aienv::commands::shell::build_shell_args;

    let args = build_shell_args(
        std::path::Path::new("/bin/fish"),
        std::path::Path::new("/root"),
        std::path::Path::new("env.zsh"),
    );
    assert_eq!(args[0], "/bin/fish");
    assert_eq!(args[1], "-c");
    assert!(args[2].contains("source"));
    assert!(args[2].contains("exec"));
    assert!(args[2].contains("-i"));
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
