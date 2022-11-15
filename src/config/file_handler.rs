use crate::config::error::ConfigError;

use serde::{de::DeserializeOwned, Serialize};
use std::{io, path::PathBuf};
use tokio::{fs, fs::File, io::AsyncWriteExt};

pub struct ConfigFileHandler<C> {
    config_type: std::marker::PhantomData<C>,
    path: PathBuf,
}

impl<C: Serialize + DeserializeOwned + Default + Clone> ConfigFileHandler<C> {
    pub async fn from_config_file(config_name: &str) -> Result<Self, ConfigError> {
        let config_file_name = format!("{}.toml", config_name);
        let mut path = Self::ensure_config_dir_exists().await?;
        path.push(config_file_name);

        Ok(Self {
            config_type: std::marker::PhantomData,
            path,
        })
    }

    pub async fn read(&mut self) -> Result<C, ConfigError> {
        match fs::read_to_string(&self.path).await {
            Ok(contents) => Ok(toml::from_str(&contents)?),
            Err(error) => match error.kind() {
                io::ErrorKind::NotFound => {
                    let config = Default::default();
                    Self::write_to_path(&self.path, &config).await?;
                    Ok(config)
                }
                _ => Err(ConfigError::ReadConfigFile),
            },
        }
    }

    pub async fn write(&self, config: &C) -> Result<(), ConfigError> {
        Self::write_to_path(&self.path, config).await
    }

    async fn write_to_path(path: &PathBuf, config: &C) -> Result<(), ConfigError> {
        let toml = toml::to_string(config)?;
        let mut file = File::create(path)
            .await
            .map_err(ConfigError::CreateConfigFile)?;
        file.write(toml.as_bytes())
            .await
            .map_err(ConfigError::WriteConfigFile)?;
        file.flush().await.map_err(ConfigError::WriteConfigFile)?;
        Ok(())
    }

    async fn ensure_config_dir_exists() -> Result<PathBuf, ConfigError> {
        let dir = Self::find_config_dir()?;
        fs::create_dir_all(&dir)
            .await
            .map_err(ConfigError::CreateConfigDir)?;

        Ok(dir)
    }

    fn find_config_dir() -> Result<PathBuf, ConfigError> {
        let mut path = PathBuf::new();

        match std::env::var("XDG_CONFIG_HOME") {
            Ok(config_dir) => path.push(config_dir),
            _ => {
                let home = std::env::var("HOME")?;
                path.push(home);
                path.push(".config");
            }
        }

        path.push("tuibe");
        Ok(path)
    }
}
