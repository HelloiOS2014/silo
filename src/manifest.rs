use crate::error::AienvError;
use serde::Deserialize;
use std::{collections::BTreeMap, path::PathBuf};

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
    pub secrets: SecretsConfig,
    pub shell: ShellConfig,
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
    pub provider: String,
    #[serde(default)]
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShellConfig {
    pub program: PathBuf,
    pub init: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkConfig {
    pub mode: String,
}

fn default_true() -> bool {
    true
}

impl Manifest {
    pub fn parse(raw: &str) -> Result<Self, AienvError> {
        let manifest: Self = toml::from_str(raw)?;
        manifest.validate()?;
        Ok(manifest)
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
            return Err(AienvError::ManifestValidation(
                "id cannot be empty".into(),
            ));
        }

        Ok(())
    }
}
