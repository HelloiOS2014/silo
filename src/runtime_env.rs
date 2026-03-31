use crate::manifest::Manifest;
use std::{collections::BTreeMap, path::Path};

pub fn build_child_env(
    manifest: &Manifest,
    host: &BTreeMap<String, String>,
    secrets: BTreeMap<String, String>,
    exec_dir: Option<&str>,
) -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();

    // 1. Allow list filtered by deny
    for key in &manifest.env.allow {
        if manifest.env.deny.contains(key) {
            continue;
        }
        if let Some(value) = host.get(key) {
            env.insert(key.clone(), value.clone());
        }
    }

    // 2. env.set
    for (key, value) in &manifest.env.set {
        env.insert(key.clone(), value.clone());
    }

    // 3. secrets
    for (key, value) in secrets {
        env.insert(key, value);
    }

    // 4. network mode
    let proxy_url = match manifest.network.mode.as_str() {
        "offline" => Some("http://127.0.0.1:1".to_string()),
        "proxy" => manifest.network.proxy_url.clone(),
        _ => None,
    };
    if let Some(url) = proxy_url {
        env.insert("http_proxy".into(), url.clone());
        env.insert("https_proxy".into(), url.clone());
        env.insert("ALL_PROXY".into(), url);
    }

    // 5. exec dir
    if let Some(dir) = exec_dir {
        env.insert("AIENV_EXEC_DIR".into(), dir.to_string());
    }

    // 6. Forced variables (last — cannot be overridden)
    env.insert("HOME".into(), join_str(&manifest.root, "home"));
    env.insert("XDG_CONFIG_HOME".into(), join_str(&manifest.root, "config"));
    env.insert("XDG_CACHE_HOME".into(), join_str(&manifest.root, "cache"));
    env.insert("XDG_DATA_HOME".into(), join_str(&manifest.root, "data"));
    env.insert("XDG_STATE_HOME".into(), join_str(&manifest.root, "state"));
    env.insert("TMPDIR".into(), join_str(&manifest.root, "tmp"));

    let aienv_root = host.get("AIENV_ROOT").cloned().unwrap_or_else(|| {
        host.get("HOME")
            .map(|h| format!("{h}/.aienv"))
            .unwrap_or_default()
    });
    env.insert("AIENV_ROOT".into(), aienv_root);

    env
}

fn join_str(root: &Path, child: &str) -> String {
    root.join(child).to_string_lossy().into_owned()
}
