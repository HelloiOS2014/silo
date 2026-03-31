use crate::error::AienvError;
use serde::Deserialize;
use std::{collections::BTreeMap, path::PathBuf};

const RESERVED_KEYS: &[&str] = &[
    "HOME",
    "XDG_CONFIG_HOME",
    "XDG_CACHE_HOME",
    "XDG_DATA_HOME",
    "XDG_STATE_HOME",
    "TMPDIR",
    "SILO_ROOT",
];

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub id: String,
    pub root: PathBuf,
    #[serde(default = "default_true")]
    pub inherit_cwd: bool,
    #[serde(default)]
    pub shared_paths: Vec<PathBuf>,
    pub env: EnvConfig,
    #[serde(default)]
    pub secrets: SecretsConfig,
    #[serde(default)]
    pub shell: ShellConfig,
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub extends: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct EnvConfig {
    #[serde(default)]
    pub allow: Vec<String>,
    #[serde(default)]
    pub deny: Vec<String>,
    #[serde(default)]
    pub set: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecretsConfig {
    #[serde(default = "default_none_provider")]
    pub provider: String,
    #[serde(default)]
    pub items: Vec<String>,
}

impl Default for SecretsConfig {
    fn default() -> Self {
        Self {
            provider: "none".into(),
            items: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShellConfig {
    #[serde(default = "default_zsh")]
    pub program: PathBuf,
    #[serde(default = "default_env_zsh")]
    pub init: PathBuf,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            program: PathBuf::from("/bin/zsh"),
            init: PathBuf::from("env.zsh"),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkConfig {
    #[serde(default = "default_network_mode")]
    pub mode: String,
    #[serde(default)]
    pub proxy_url: Option<String>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            mode: "default".into(),
            proxy_url: None,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_none_provider() -> String {
    "none".into()
}

fn default_zsh() -> PathBuf {
    PathBuf::from("/bin/zsh")
}

fn default_env_zsh() -> PathBuf {
    PathBuf::from("env.zsh")
}

fn default_network_mode() -> String {
    "default".into()
}

fn expand_tilde_path(path: &std::path::Path, home: &str) -> PathBuf {
    let s = path.to_string_lossy();
    if s == "~" {
        PathBuf::from(home)
    } else if let Some(rest) = s.strip_prefix("~/") {
        PathBuf::from(home).join(rest)
    } else {
        path.to_path_buf()
    }
}

impl Manifest {
    pub fn parse(raw: &str) -> Result<Self, AienvError> {
        let mut manifest: Self = toml::from_str(raw)?;
        manifest.expand_tilde();
        manifest.validate()?;
        Ok(manifest)
    }

    fn expand_tilde(&mut self) {
        if let Ok(home) = std::env::var("HOME") {
            self.root = expand_tilde_path(&self.root, &home);

            self.shared_paths = self
                .shared_paths
                .iter()
                .map(|p| expand_tilde_path(p, &home))
                .collect();
        }
    }

    pub fn validate(&self) -> Result<(), AienvError> {
        if self.extends.is_some() {
            return Err(AienvError::ManifestValidation(
                "manifest inheritance via `extends` is not implemented yet".into(),
            ));
        }

        match self.network.mode.as_str() {
            "default" | "offline" | "proxy" => {}
            other => {
                return Err(AienvError::ManifestValidation(format!(
                    "network.mode must be default|offline|proxy, got {other}"
                )));
            }
        }

        if self.id.trim().is_empty() {
            return Err(AienvError::ManifestValidation("id cannot be empty".into()));
        }

        // Validate secrets.provider
        match self.secrets.provider.as_str() {
            "keychain" | "envfile" | "none" => {}
            other => {
                return Err(AienvError::ManifestValidation(format!(
                    "secrets.provider must be keychain|envfile|none, got {other}"
                )));
            }
        }

        // Validate reserved keys in env.set
        for key in self.env.set.keys() {
            if RESERVED_KEYS.contains(&key.as_str()) {
                return Err(AienvError::ManifestValidation(format!(
                    "env.set contains reserved key: {key}"
                )));
            }
        }

        // Validate reserved keys in secrets.items
        for key in &self.secrets.items {
            if RESERVED_KEYS.contains(&key.as_str()) {
                return Err(AienvError::ManifestValidation(format!(
                    "secrets.items contains reserved key: {key}"
                )));
            }
        }

        // Validate proxy_url required when mode = "proxy"
        if self.network.mode == "proxy" && self.network.proxy_url.is_none() {
            return Err(AienvError::ManifestValidation(
                "network.mode = \"proxy\" requires proxy_url to be set".into(),
            ));
        }

        Ok(())
    }
}
