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
fn defaults_missing_fields() {
    let raw = r#"
id = "work"
root = "/tmp/work"

[env]

[secrets]
provider = "keychain"

[shell]
program = "/bin/zsh"
init = "env.zsh"

[network]
mode = "default"
"#;

    let manifest = Manifest::parse(raw).unwrap();
    assert!(manifest.inherit_cwd);
    assert!(manifest.shared_paths.is_empty());
    assert!(manifest.env.allow.is_empty());
    assert!(manifest.env.deny.is_empty());
    assert!(manifest.env.set.is_empty());
    assert!(manifest.secrets.items.is_empty());
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

#[test]
fn rejects_empty_id() {
    let raw = r#"
id = ""
root = "/tmp/work"

[env]
allow = []
deny = []

[secrets]
provider = "keychain"

[shell]
program = "/bin/zsh"
init = "env.zsh"

[network]
mode = "default"
"#;

    let err = Manifest::parse(raw).unwrap_err();
    assert!(err.to_string().contains("id cannot be empty"));
}

#[test]
fn rejects_unknown_fields() {
    let raw = r#"
id = "work"
root = "/tmp/work"
typo = true

[env]
allow = []
deny = []

[secrets]
provider = "keychain"

[shell]
program = "/bin/zsh"
init = "env.zsh"

[network]
mode = "default"
"#;

    let err = Manifest::parse(raw).unwrap_err();
    assert!(err.to_string().contains("unknown field"));
}

#[test]
fn rejects_extends_until_inheritance_is_implemented() {
    let raw = r#"
id = "work"
root = "/tmp/work"
extends = "base/default"

[env]
allow = []
deny = []

[secrets]
provider = "keychain"

[shell]
program = "/bin/zsh"
init = "env.zsh"

[network]
mode = "default"
"#;

    let err = Manifest::parse(raw).unwrap_err();
    assert!(err.to_string().contains("extends"));
}

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
        std::path::PathBuf::from(&home).join(".aienv/work")
    );
}
