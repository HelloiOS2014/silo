use crate::env_path;
use anyhow::Result;
use std::fs;

pub fn run() -> Result<()> {
    let root = env_path::silo_root()?;
    if !root.exists() {
        return Ok(());
    }

    let mut names: Vec<String> = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if entry.path().join("manifest.toml").exists() {
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
    }

    names.sort();
    for name in names {
        println!("{name}");
    }

    Ok(())
}
