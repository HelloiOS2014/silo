use crate::manifest::Manifest;
use std::{collections::BTreeMap, path::Path};

pub fn build_child_env(
    manifest: &Manifest,
    host: &BTreeMap<String, String>,
    secrets: BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();

    for key in &manifest.env.allow {
        if manifest.env.deny.contains(key) {
            continue;
        }

        if let Some(value) = host.get(key) {
            env.insert(key.clone(), value.clone());
        }
    }

    env.insert("HOME".into(), join_str(&manifest.root, "home"));
    env.insert("XDG_CONFIG_HOME".into(), join_str(&manifest.root, "config"));
    env.insert("XDG_CACHE_HOME".into(), join_str(&manifest.root, "cache"));
    env.insert("TMPDIR".into(), join_str(&manifest.root, "tmp"));

    for (key, value) in &manifest.env.set {
        env.insert(key.clone(), value.clone());
    }

    for (key, value) in secrets {
        env.insert(key, value);
    }

    env
}

fn join_str(root: &Path, child: &str) -> String {
    root.join(child).to_string_lossy().into_owned()
}
