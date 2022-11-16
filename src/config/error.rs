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

    #[error(display = "Failed to fetch RSS feed")]
    FetchFeed(#[error(from)] reqwest::Error),

    #[error(display = "Failed to join fetch handles")]
    JoinFetchTasks(#[error(from)] tokio::task::JoinError),

    #[error(display = "Failed to read RSS feed: {}", _0)]
    ReadFeed(#[error(from)] atom_syndication::Error),

    #[error(display = "Failed to parse video")]
    ParseVideo,

    #[error(display = "Failed to read subscriptions file: {}", _0)]
    ReadYoutubeTakeout(#[error(source, no_from)] std::io::Error),

    #[error(display = "Failed to parse YouTube takeout")]
    ParseYoutubeTakeout,
}
