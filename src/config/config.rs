use crate::{config::error::ConfigError, video::Video};
use async_trait::async_trait;
use std::collections::{BTreeSet, HashMap};
use tokio::sync::oneshot;

pub type ConfigUpdate = Result<ConfigData, ConfigError>;

#[derive(Clone)]
pub struct ConfigData {
    pub channels: HashMap<String, String>,
    pub videos: BTreeSet<Video>,
}

#[async_trait]
pub trait Config {
    async fn load() -> Result<Self, ConfigError>
    where
        Self: Sized;
    fn fetch(&self) -> oneshot::Receiver<ConfigUpdate>;
    fn add_channel(&self, id: String) -> oneshot::Receiver<ConfigUpdate>;
    fn remove_subscription(&self, id: String) -> oneshot::Receiver<ConfigUpdate>;
    fn data(&self) -> ConfigData;
}
