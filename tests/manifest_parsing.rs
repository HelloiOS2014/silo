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
