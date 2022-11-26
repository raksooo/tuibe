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

    fn subscribe(&self) -> config_message_channel::ConfigReceiver;
    fn refetch(&self);
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct Video {
    date: Reverse<DateTime<FixedOffset>>,
    pub title: String,
    pub url: String,
    pub author: String,
    pub description: String,
    pub length: u32,
}

impl Video {
    pub fn date(&self) -> DateTime<FixedOffset> {
        self.date.0
    }

    pub fn label(&self, width: usize) -> String {
        // Subtract width of datetime and horizontal padding and checkmark.
        let width = width - 16 - 2 - 2;
        // Split the area between title and author 3/4 for author.
        let title_width = 3 * width / 4;
        let author_width = width - title_width;

        let date = self.date.0.format("%Y-%m-%d %H:%M");
        let title = self.title.get(..title_width).unwrap_or(&self.title);
        let author = self.author.get(..author_width).unwrap_or(&self.author);

        format!("{title:title_width$} {author:>author_width$} - {date} ")
    }
}
