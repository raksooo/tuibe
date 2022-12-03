pub mod common;
pub mod error;
pub mod rss;

pub mod config_message_channel;
mod file_handler;

use crate::config::error::ConfigError;

use async_trait::async_trait;
use chrono::{DateTime, FixedOffset};
use std::cmp::Reverse;

#[async_trait]
pub trait Config {
    async fn load() -> Result<Self, ConfigError>
    where
        Self: Sized;

    fn subscribe(&self) -> config_message_channel::ConfigReceiver<Video>;
    fn refetch(&self);
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct Video {
    date: Reverse<DateTime<FixedOffset>>,
    pub title: String,
    pub url: String,
    pub author: String,
    pub feed_url: String,
    pub description: String,
    pub length: u32,
}

impl Video {
    pub fn date(&self) -> DateTime<FixedOffset> {
        self.date.0
    }
}
