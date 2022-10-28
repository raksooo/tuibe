use crate::config::error::ConfigError;
use async_trait::async_trait;
use chrono::{DateTime, FixedOffset};
use tokio::sync::oneshot;

pub type ConfigResult = Result<Vec<Video>, ConfigError>;

#[async_trait]
pub trait Config {
    async fn load() -> Result<Self, ConfigError>
    where
        Self: Sized;

    fn fetch(&self) -> oneshot::Receiver<ConfigResult>;
    fn videos(&self) -> Vec<Video>;
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct Video {
    pub date: DateTime<FixedOffset>,
    pub title: String,
    pub url: String,
    pub author: String,
    pub description: String,
    pub length: u32,
}

impl Video {
    pub fn get_label(&self, width: usize) -> String {
        // Subtract width of datetime and horizontal padding and checkmark.
        let width = width - 16 - 2 - 2;
        // Split the area between title and author 3/4 for author.
        let title_width = 3 * width / 4;
        let author_width = width - title_width;

        let date = self.date.format("%Y-%m-%d %H:%M");
        let title = self.title.get(..title_width).unwrap_or(&self.title);
        let author = self.author.get(..author_width).unwrap_or(&self.author);

        format!("{title:title_width$} {author:>author_width$} - {date} ")
    }
}
