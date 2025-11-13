pub mod rss;

pub mod channel;

use crate::config_error::ConfigError;
use rss::RssBackendError;

use async_trait::async_trait;
use chrono::{DateTime, FixedOffset};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BackendError {
    #[error("Config error: {}", _0)]
    ConfigError(#[from] ConfigError),

    #[error("RSS backend error: {}", _0)]
    RssBackendError(#[from] RssBackendError),
}

#[async_trait]
pub trait Backend {
    async fn load() -> Result<Self, BackendError>
    where
        Self: Sized;

    fn subscribe(&self) -> channel::BackendReceiver<Video>;
    fn refetch(&self);
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct Video {
    pub date: DateTime<FixedOffset>,
    pub title: String,
    pub url: String,
    pub author: String,
    pub feed_url: String,
    pub description: String,
    pub length: u32,
}
