use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to find config dir")]
    FindConfigDir(#[from] std::env::VarError),

    #[error("Failed to read config file")]
    ReadConfigFile,

    #[error("Failed to parse config file: {}", _0)]
    ParseConfigFile(#[from] toml::de::Error),

    #[error("Failed to create config directory")]
    CreateConfigDir(#[source] std::io::Error),

    #[error("Failed to create/truncate config file")]
    CreateConfigFile(#[source] std::io::Error),

    #[error("Failed to serialize config")]
    SerializeConfig(#[from] toml::ser::Error),

    #[error("Failed to write to config file")]
    WriteConfigFile(#[source] std::io::Error),
}
