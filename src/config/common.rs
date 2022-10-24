use super::file_handler::ConfigFileHandler;
use crate::config::error::ConfigError;
use chrono::Utc;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::oneshot;

const CONFIG_NAME: &str = "config";

#[derive(Clone, Serialize, Deserialize)]
pub struct CommonConfig {
    pub player: String,
    pub last_played_timestamp: i64,
}

impl Default for CommonConfig {
    fn default() -> Self {
        Self {
            player: "mpv".to_string(),
            last_played_timestamp: Utc::now().timestamp(),
        }
    }
}

pub struct CommonConfigHandler {
    pub config: Arc<Mutex<CommonConfig>>,
    file_handler: Arc<tokio::sync::Mutex<ConfigFileHandler>>,
}

impl CommonConfigHandler {
    pub async fn load() -> Result<Self, ConfigError> {
        let mut file_handler = ConfigFileHandler::from_config_file(CONFIG_NAME).await?;
        let config = file_handler.read().await?;

        Ok(Self {
            config: Arc::new(Mutex::new(config)),
            file_handler: Arc::new(tokio::sync::Mutex::new(file_handler)),
        })
    }

    pub async fn set_player(&self, player: String) -> oneshot::Receiver<Result<(), ConfigError>> {
        self.modify(move |mut config| {
            config.player = player;
            config
        })
    }

    pub async fn set_last_played_timestamp(
        &self,
        last_played_timestamp: i64,
    ) -> oneshot::Receiver<Result<(), ConfigError>> {
        self.modify(move |mut config| {
            config.last_played_timestamp = last_played_timestamp;
            config
        })
    }

    pub fn config(&self) -> CommonConfig {
        let config = self.config.lock();
        config.clone()
    }

    fn modify<F>(&self, f: F) -> oneshot::Receiver<Result<(), ConfigError>>
    where
        F: FnOnce(CommonConfig) -> CommonConfig + Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        let config = Arc::clone(&self.config);
        let file_handler = Arc::clone(&self.file_handler);

        tokio::spawn(async move {
            let new_data = Self::modify_impl(config, file_handler, f).await;
            let _ = tx.send(new_data);
        });

        rx
    }

    async fn modify_impl<F>(
        config: Arc<Mutex<CommonConfig>>,
        file_handler: Arc<tokio::sync::Mutex<ConfigFileHandler>>,
        f: F,
    ) -> Result<(), ConfigError>
    where
        F: FnOnce(CommonConfig) -> CommonConfig + Send + 'static,
    {
        let new_config = {
            let config = config.lock();
            f(config.clone())
        };

        let file_handler = file_handler.lock().await;
        file_handler.write(&new_config).await
    }
}
