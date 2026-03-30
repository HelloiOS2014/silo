use thiserror::Error;

#[derive(Debug, Error)]
pub enum AienvError {
    #[error("manifest parse error: {0}")]
    ManifestParse(#[from] toml::de::Error),
    #[error("manifest validation error: {0}")]
    ManifestValidation(String),
}
