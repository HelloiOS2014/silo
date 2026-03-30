use anyhow::{bail, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn validate_cwd(cwd: &Path, shared_paths: &[PathBuf]) -> Result<PathBuf> {
    let cwd_real = fs::canonicalize(cwd).map_err(|e| anyhow::anyhow!("cwd is invalid: {e}"))?;

    if shared_paths.is_empty() {
        return Ok(cwd_real);
    }

    for path in shared_paths {
        let shared_real =
            fs::canonicalize(path).map_err(|e| anyhow::anyhow!("shared path is invalid: {e}"))?;

        if cwd_real.starts_with(&shared_real) {
            return Ok(cwd_real);
        }
    }

    bail!("cwd is outside allowed paths")
}
