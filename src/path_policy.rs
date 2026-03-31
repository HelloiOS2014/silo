use anyhow::Result;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn validate_cwd(cwd: &Path, shared_paths: &[PathBuf]) -> Result<(PathBuf, Vec<PathBuf>)> {
    let cwd_real = fs::canonicalize(cwd).map_err(|e| anyhow::anyhow!("cwd is invalid: {e}"))?;

    let mut shared_real = Vec::new();
    for path in shared_paths {
        let canonical =
            fs::canonicalize(path).map_err(|e| anyhow::anyhow!("shared path is invalid: {e}"))?;
        shared_real.push(canonical);
    }

    Ok((cwd_real, shared_real))
}
