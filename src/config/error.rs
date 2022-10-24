use err_derive::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error(display = "Failed to find config dir")]
    FindConfigDir,

    #[error(display = "Failed to read config file")]
    ReadConfigFile,

    #[error(display = "Failed to parse config file: {:?}", error)]
    ParseConfigFile {
        #[error(from)]
        error: toml::de::Error,
    },

    #[error(display = "Failed to create config directory")]
    CreateConfigDir,

    #[error(display = "Failed to create/truncate config file")]
    CreateConfigFile,

    #[error(display = "Failed to serialize config")]
    SerializeConfig,

    #[error(display = "Failed to write to config file")]
    WriteConfigFile,

    #[error(display = "Can't remove subscription since it doesn't exist")]
    SubscriptionDoesNotExist,

    #[error(display = "Feed error: {:?}", error)]
    FeedError {
        #[error(from)]
        error: FeedError,
    },
}

#[derive(Debug, Error)]
pub enum FeedError {
    #[error(display = "Failed to fetch RSS feed")]
    FetchFeed,

    #[error(display = "Failed to read RSS feed: {:?}", error)]
    ReadFeed { error: atom_syndication::Error },

    #[error(display = "Failed to parse video")]
    ParseVideo,
}