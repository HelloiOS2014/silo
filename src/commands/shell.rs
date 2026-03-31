use anyhow::Result;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{env_path, path_policy::validate_cwd, runtime_env::build_child_env};

pub fn run(env: &str, cwd: Option<PathBuf>) -> Result<i32> {
    let (manifest, env_root) = env_path::load_manifest(env)?;
    let secrets = env_path::resolve_secrets(&manifest, &env_root)?;

    let cwd = match cwd {
        Some(c) => c,
        None => {
            if manifest.inherit_cwd {
                std::env::current_dir()?
            } else {
                manifest.root.join("home")
            }
        }
    };
    let (cwd, _shared) = validate_cwd(&cwd, &manifest.shared_paths)?;

    let pid = std::process::id();
    let run_dir = env_root.join("run").join(pid.to_string());
    fs::create_dir_all(&run_dir)?;
    let run_dir_str = run_dir.to_string_lossy().to_string();

    let host: BTreeMap<String, String> = std::env::vars().collect();
    let child_env = build_child_env(&manifest, &host, secrets, Some(&run_dir_str));

    let args = build_shell_args(
        &manifest.shell.program,
        &manifest.root,
        &manifest.shell.init,
    );

    let status = Command::new(&args[0])
        .args(&args[1..])
        .current_dir(&cwd)
        .env_clear()
        .envs(child_env)
        .status()?;

    let _ = fs::remove_dir_all(&run_dir);

    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        Ok(status
            .code()
            .unwrap_or_else(|| 128 + status.signal().unwrap_or(1)))
    }
    #[cfg(not(unix))]
    {
        Ok(status.code().unwrap_or(1))
    }
}

/// Build shell launch arguments with rc-file suppression.
pub fn build_shell_args(program: &Path, env_root: &Path, init: &Path) -> Vec<String> {
    let program_str = program.to_string_lossy().to_string();
    let init_path = env_root.join(init);
    let init_str = init_path.to_string_lossy().to_string();

    let shell_name = program
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let (suppress_flags, exec_cmd) = match shell_name.as_str() {
        "zsh" => (
            vec!["--no-globalrcs".to_string(), "--no-rcs".to_string()],
            format!("source \"{init_str}\" && exec \"{program_str}\" --no-globalrcs --no-rcs -i"),
        ),
        "bash" => (
            vec!["--noprofile".to_string(), "--norc".to_string()],
            format!("source \"{init_str}\" && exec \"{program_str}\" --noprofile --norc -i"),
        ),
        _ => (
            Vec::new(),
            format!("source \"{init_str}\" && exec \"{program_str}\" -i"),
        ),
    };

    let mut args = vec![program_str];
    args.extend(suppress_flags);
    args.push("-c".to_string());
    args.push(exec_cmd);
    args
}
