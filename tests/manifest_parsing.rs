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
