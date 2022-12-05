use err_derive::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error(display = "Failed to find config dir")]
    FindConfigDir(#[error(from)] std::env::VarError),

    #[error(display = "Failed to read config file")]
    ReadConfigFile,

    #[error(display = "Failed to parse config file: {}", _0)]
    ParseConfigFile(#[error(from)] toml::de::Error),

    #[error(display = "Failed to create config directory")]
    CreateConfigDir(#[error(source, no_from)] std::io::Error),

    #[error(display = "Failed to create/truncate config file")]
    CreateConfigFile(#[error(source, no_from)] std::io::Error),

    #[error(display = "Failed to serialize config")]
    SerializeConfig(#[error(from)] toml::ser::Error),

    #[error(display = "Failed to write to config file")]
    WriteConfigFile(#[error(source, no_from)] std::io::Error),
}
