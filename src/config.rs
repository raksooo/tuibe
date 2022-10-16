use crate::error::ConfigError;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{io, path::PathBuf};
use tokio::{fs, fs::File, io::AsyncWriteExt};

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub player: String,
    pub last_played_timestamp: i64,
    pub subscriptions: Vec<String>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            player: "mpv".to_string(),
            last_played_timestamp: Utc::now().timestamp(),
            subscriptions: Vec::new(),
        }
    }
}

pub struct ConfigHandler {
    path: PathBuf,
    pub config: Config,
}

impl ConfigHandler {
    pub async fn load() -> Result<ConfigHandler, ConfigError> {
        let mut path = Self::ensure_config_dir_exists().await?;
        path.push("config.toml");

        let config = Self::read_config(&path).await?;

        Ok(ConfigHandler { path, config })
    }

    pub async fn set_player(&mut self, player: &str) -> Result<(), ConfigError> {
        self.config.player = player.to_string();
        self.write_config(&self.config).await
    }

    pub async fn set_last_played_timestamp(
        &mut self,
        last_played_timestamp: i64,
    ) -> Result<(), ConfigError> {
        self.config.last_played_timestamp = last_played_timestamp;
        self.write_config(&self.config).await
    }

    pub async fn add_subscription(&mut self, subscription: String) -> Result<(), ConfigError> {
        self.config.subscriptions.push(subscription);
        self.write_config(&self.config).await
    }

    pub async fn remove_subscription(&mut self, subscription: String) -> Result<(), ConfigError> {
        let index = self
            .config
            .subscriptions
            .iter()
            .position(|item| *item == subscription)
            .ok_or(ConfigError::SubscriptionDoesNotExist)?;
        self.config.subscriptions.remove(index);
        self.write_config(&self.config).await
    }

    async fn read_config(path: &PathBuf) -> Result<Config, ConfigError> {
        match fs::read_to_string(path).await {
            Ok(contents) => toml::from_str(&contents).map_err(|_| ConfigError::ParseConfigFile),
            Err(error) => match error.kind() {
                io::ErrorKind::NotFound => {
                    let config = Default::default();
                    Self::write_config_to_path(path, &config)
                        .await
                        .map_err(|_| ConfigError::WriteConfigFile)?;
                    Ok(config)
                }
                _ => Err(ConfigError::ReadConfigFile),
            },
        }
    }

    async fn write_config(&self, config: &Config) -> Result<(), ConfigError> {
        Self::write_config_to_path(&self.path, config).await
    }

    async fn write_config_to_path(path: &PathBuf, config: &Config) -> Result<(), ConfigError> {
        let toml = toml::to_string(config).map_err(|_| ConfigError::SerializeConfig)?;
        let mut file = File::create(path)
            .await
            .map_err(|_| ConfigError::CreateConfigFile)?;
        file.write(toml.as_bytes())
            .await
            .map_err(|_| ConfigError::WriteConfigFile)?;
        file.flush()
            .await
            .map_err(|_| ConfigError::WriteConfigFile)?;
        Ok(())
    }

    async fn ensure_config_dir_exists() -> Result<PathBuf, ConfigError> {
        let dir = Self::find_config_dir()?;
        fs::create_dir_all(&dir)
            .await
            .map_err(|_| ConfigError::CreateConfigDir)?;

        Ok(dir)
    }

    fn find_config_dir() -> Result<PathBuf, ConfigError> {
        let mut path = PathBuf::new();

        match std::env::var("XDG_CONFIG_HOME") {
            Ok(config_dir) => path.push(config_dir),
            _ => {
                let home = std::env::var("HOME").map_err(|_| ConfigError::FindConfigDir)?;
                path.push(home);
                path.push(".config".to_string());
            }
        }

        path.push("youtuibe");
        Ok(path)
    }
}
