use super::file_handler::ConfigFileHandler;
use crate::config::error::ConfigError;

use chrono::Utc;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

const CONFIG_NAME: &str = "config";

#[derive(Clone, Serialize, Deserialize)]
pub struct CommonConfig {
    pub player: String,
    pub last_played_timestamp: i64,
}

impl Default for CommonConfig {
    fn default() -> Self {
        Self {
            player: String::from("mpv"),
            last_played_timestamp: Utc::now().timestamp(),
        }
    }
}

pub struct CommonConfigHandler {
    pub config: Mutex<CommonConfig>,
    file_handler: tokio::sync::Mutex<ConfigFileHandler<CommonConfig>>,
}

impl CommonConfigHandler {
    pub async fn load() -> Result<Self, ConfigError> {
        let mut file_handler = ConfigFileHandler::from_config_file(CONFIG_NAME).await?;
        let config = file_handler.read().await?;

        Ok(Self {
            config: Mutex::new(config),
            file_handler: tokio::sync::Mutex::new(file_handler),
        })
    }

    pub async fn set_last_played_timestamp(
        &self,
        last_played_timestamp: i64,
    ) -> Result<(), ConfigError> {
        let new_config = {
            let mut config = self.config.lock();
            config.last_played_timestamp = last_played_timestamp;
            config.clone()
        };

        let file_handler = self.file_handler.lock().await;
        file_handler.write(&new_config).await
    }

    pub fn player(&self) -> String {
        self.config.lock().player.clone()
    }

    pub fn last_played_timestamp(&self) -> i64 {
        self.config.lock().last_played_timestamp
    }
}
