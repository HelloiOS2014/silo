use anyhow::Result;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn validate_cwd(cwd: &Path, shared_paths: &[PathBuf]) -> Result<PathBuf> {
    let cwd_real = fs::canonicalize(cwd).map_err(|e| anyhow::anyhow!("cwd is invalid: {e}"))?;

    for path in shared_paths {
        let _shared_real =
            fs::canonicalize(path).map_err(|e| anyhow::anyhow!("shared path is invalid: {e}"))?;
    }

    Ok(cwd_real)
}
