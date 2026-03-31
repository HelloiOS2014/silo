use crate::env_path;
use anyhow::Result;

pub fn run(env: &str) -> Result<()> {
    let (manifest, env_root) = env_path::load_manifest(env)?;

    println!("Environment:     {}", manifest.id);
    println!("Root:            {}", manifest.root.display());
    println!("Inherit CWD:     {}", manifest.inherit_cwd);
    println!("Network:         {}", manifest.network.mode);
    if let Some(url) = &manifest.network.proxy_url {
        println!("Proxy URL:       {url}");
    }
    println!();

    if manifest.env.allow.is_empty() {
        println!("Env Allow:       (none)");
    } else {
        println!("Env Allow:       {}", manifest.env.allow.join(", "));
    }
    if manifest.env.deny.is_empty() {
        println!("Env Deny:        (none)");
    } else {
        println!("Env Deny:        {}", manifest.env.deny.join(", "));
    }
    if manifest.env.set.is_empty() {
        println!("Env Set:         (none)");
    } else {
        let pairs: Vec<String> = manifest
            .env
            .set
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        println!("Env Set:         {}", pairs.join(", "));
    }
    println!();

    let provider_display = match manifest.secrets.provider.as_str() {
        "keychain" => format!("keychain (aienv.{})", manifest.id),
        "envfile" => format!("envfile ({})", env_root.join("secrets.env").display()),
        other => other.to_string(),
    };
    println!("Secrets:         {provider_display}");
    if manifest.secrets.items.is_empty() {
        println!("Secret Items:    (none)");
    } else {
        println!("Secret Items:    {}", manifest.secrets.items.join(", "));
    }
    println!();

    println!("Shell:           {}", manifest.shell.program.display());
    println!("Shell Init:      {}", manifest.shell.init.display());
    println!();

    println!("Directories:");
    println!("  HOME             {}", manifest.root.join("home").display());
    println!("  XDG_CONFIG_HOME  {}", manifest.root.join("config").display());
    println!("  XDG_CACHE_HOME   {}", manifest.root.join("cache").display());
    println!("  XDG_DATA_HOME    {}", manifest.root.join("data").display());
    println!("  XDG_STATE_HOME   {}", manifest.root.join("state").display());
    println!("  TMPDIR           {}", manifest.root.join("tmp").display());

    Ok(())
}
