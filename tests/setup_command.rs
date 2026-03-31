use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn create_env_with_setup(home: &std::path::Path, on_init: &[&str]) -> std::path::PathBuf {
    let env_root = home.join(".silo").join("test-setup");
    for dir in ["home", "config", "cache", "data", "state", "tmp", "run"] {
        fs::create_dir_all(env_root.join(dir)).unwrap();
    }

    let on_init_toml: Vec<String> = on_init.iter().map(|s| format!("  \"{s}\",")).collect();
    let manifest = format!(
        "id = \"test-setup\"\n\
         root = \"{root}\"\n\
         inherit_cwd = true\n\
         shared_paths = []\n\
         \n\
         [env]\n\
         allow = [\"PATH\"]\n\
         deny = []\n\
         \n\
         [env.set]\n\
         AI_ENV = \"test-setup\"\n\
         \n\
         [secrets]\n\
         provider = \"none\"\n\
         items = []\n\
         \n\
         [shell]\n\
         program = \"/bin/zsh\"\n\
         init = \"env.zsh\"\n\
         \n\
         [network]\n\
         mode = \"default\"\n\
         \n\
         [setup]\n\
         on_init = [\n\
         {on_init}\n\
         ]\n",
        root = env_root.display(),
        on_init = on_init_toml.join("\n"),
    );

    fs::write(env_root.join("manifest.toml"), manifest).unwrap();
    fs::write(env_root.join("env.zsh"), "export AI_ENV=test-setup\n").unwrap();

    let secrets_path = env_root.join("secrets.env");
    fs::write(&secrets_path, "").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&secrets_path, fs::Permissions::from_mode(0o600)).unwrap();
    }

    env_root
}

#[test]
fn setup_runs_on_init_commands_in_isolated_env() {
    let home = TempDir::new().unwrap();
    let env_root = create_env_with_setup(home.path(), &["touch $HOME/setup-marker"]);

    let mut cmd = Command::cargo_bin("silo").unwrap();
    cmd.env("HOME", home.path())
        .args(["setup", "--env", "test-setup"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("setup complete"));

    assert!(env_root.join("home/setup-marker").exists());
    assert!(env_root.join(".setup-done").exists());
}

#[test]
fn setup_silo_host_home_points_to_real_home() {
    let home = TempDir::new().unwrap();
    let env_root = create_env_with_setup(
        home.path(),
        &["echo $SILO_HOST_HOME > $HOME/host-home-value"],
    );

    let mut cmd = Command::cargo_bin("silo").unwrap();
    cmd.env("HOME", home.path())
        .args(["setup", "--env", "test-setup"]);

    cmd.assert().success();

    let value = fs::read_to_string(env_root.join("home/host-home-value")).unwrap();
    assert_eq!(value.trim(), home.path().to_string_lossy());
}

#[test]
fn setup_skips_when_marker_exists() {
    let home = TempDir::new().unwrap();
    let env_root = create_env_with_setup(home.path(), &["touch $HOME/setup-marker"]);

    // Write marker manually
    fs::write(env_root.join(".setup-done"), "").unwrap();

    let mut cmd = Command::cargo_bin("silo").unwrap();
    cmd.env("HOME", home.path())
        .args(["setup", "--env", "test-setup"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("already completed"));

    // Command should not have run
    assert!(!env_root.join("home/setup-marker").exists());
}

#[test]
fn setup_force_reruns_even_with_marker() {
    let home = TempDir::new().unwrap();
    let env_root = create_env_with_setup(home.path(), &["touch $HOME/setup-marker"]);

    // Write marker manually
    fs::write(env_root.join(".setup-done"), "").unwrap();

    let mut cmd = Command::cargo_bin("silo").unwrap();
    cmd.env("HOME", home.path())
        .args(["setup", "--env", "test-setup", "--force"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("setup complete"));

    // Command should have run despite marker
    assert!(env_root.join("home/setup-marker").exists());
}

#[test]
fn setup_aborts_on_command_failure() {
    let home = TempDir::new().unwrap();
    let env_root = create_env_with_setup(
        home.path(),
        &[
            "touch $HOME/first-marker",
            "false",
            "touch $HOME/third-marker",
        ],
    );

    let mut cmd = Command::cargo_bin("silo").unwrap();
    cmd.env("HOME", home.path())
        .args(["setup", "--env", "test-setup"]);

    cmd.assert().failure();

    // First command ran, third did not
    assert!(env_root.join("home/first-marker").exists());
    assert!(!env_root.join("home/third-marker").exists());
    // Marker not written on failure
    assert!(!env_root.join(".setup-done").exists());
}

#[test]
fn setup_handles_empty_on_init() {
    let home = TempDir::new().unwrap();
    let _env_root = create_env_with_setup(home.path(), &[]);

    let mut cmd = Command::cargo_bin("silo").unwrap();
    cmd.env("HOME", home.path())
        .args(["setup", "--env", "test-setup"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("no setup hooks defined"));
}

#[test]
fn manifest_without_setup_section_is_valid() {
    let home = TempDir::new().unwrap();
    let env_root = home.path().join(".silo").join("no-setup");
    for dir in ["home", "config", "cache", "data", "state", "tmp", "run"] {
        fs::create_dir_all(env_root.join(dir)).unwrap();
    }

    let manifest = format!(
        "id = \"no-setup\"\n\
         root = \"{root}\"\n\
         inherit_cwd = true\n\
         shared_paths = []\n\
         \n\
         [env]\n\
         allow = [\"PATH\"]\n\
         deny = []\n\
         \n\
         [env.set]\n\
         AI_ENV = \"no-setup\"\n\
         \n\
         [secrets]\n\
         provider = \"none\"\n\
         items = []\n\
         \n\
         [shell]\n\
         program = \"/bin/zsh\"\n\
         init = \"env.zsh\"\n\
         \n\
         [network]\n\
         mode = \"default\"\n",
        root = env_root.display(),
    );

    fs::write(env_root.join("manifest.toml"), manifest).unwrap();
    fs::write(env_root.join("env.zsh"), "").unwrap();

    let secrets_path = env_root.join("secrets.env");
    fs::write(&secrets_path, "").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&secrets_path, fs::Permissions::from_mode(0o600)).unwrap();
    }

    // setup should work — just says "no setup hooks"
    let mut cmd = Command::cargo_bin("silo").unwrap();
    cmd.env("HOME", home.path())
        .args(["setup", "--env", "no-setup"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("no setup hooks defined"));
}
