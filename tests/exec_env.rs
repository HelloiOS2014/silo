use silo::secrets::resolve_from_envfile;
use silo::{manifest::Manifest, path_policy::validate_cwd, runtime_env::build_child_env};
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
    host.insert("HOME".into(), "/Users/test".into());

    let env = build_child_env(&manifest, &host, BTreeMap::new(), None);
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
    let (validated, _) = validate_cwd(&cwd, &[shared.path().to_path_buf()]).unwrap();
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
    let env_root = home.path().join(".silo").join("work");
    fs::create_dir_all(env_root.join("home")).unwrap();
    fs::create_dir_all(env_root.join("config")).unwrap();
    fs::create_dir_all(env_root.join("cache")).unwrap();
    fs::create_dir_all(env_root.join("data")).unwrap();
    fs::create_dir_all(env_root.join("state")).unwrap();
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

    let mut cmd = Command::cargo_bin("silo").unwrap();
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

    let secrets = resolve_from_envfile(&envfile, &["KEY".into(), "KEY2".into()]).unwrap();
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

    let secrets = resolve_from_envfile(&envfile, &["KEY".into(), "KEY2".into()]).unwrap();
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

#[test]
fn forced_vars_cannot_be_overridden_by_env_set() {
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
    assert_eq!(env["SILO_ROOT"], "/Users/test/.silo");
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
fn silo_root_preserves_existing_value() {
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
    host.insert("SILO_ROOT".into(), "/custom/silo".into());

    let env = build_child_env(&manifest, &host, BTreeMap::new(), None);
    assert_eq!(env["SILO_ROOT"], "/custom/silo");
}

#[test]
fn silo_exec_dir_injected_when_provided() {
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
    assert_eq!(env["SILO_EXEC_DIR"], "/tmp/work/run/12345");
}

#[test]
fn exec_injects_envfile_secrets_into_child_process() {
    let home = TempDir::new().unwrap();
    let env_root = home.path().join(".silo").join("work");
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

    let mut cmd = Command::cargo_bin("silo").unwrap();
    cmd.env("HOME", home.path()).args([
        "exec",
        "-e",
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
    let env_root = home.path().join(".silo").join("work");
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

    let mut cmd = Command::cargo_bin("silo").unwrap();
    cmd.env("HOME", home.path()).args([
        "exec",
        "-e",
        "work",
        "--",
        "python3",
        "-c",
        "import os; print(os.getcwd())",
    ]);

    cmd.assert().success().stdout(predicate::str::contains(
        env_root.join("home").to_string_lossy().as_ref(),
    ));
}

#[test]
fn exec_rejects_mismatched_manifest_id() {
    let home = TempDir::new().unwrap();
    let env_root = home.path().join(".silo").join("work");
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

    let mut cmd = Command::cargo_bin("silo").unwrap();
    cmd.env("HOME", home.path())
        .args(["exec", "-e", "work", "--", "echo", "hi"]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("does not match"));
}
