use super::file_handler::ConfigFileHandler;
use crate::config::{error::ConfigError, Config, Video};

use async_trait::async_trait;
use atom_syndication::Entry;
use futures::future;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, sync::Arc};
use tokio::fs;

const CONFIG_NAME: &str = "rss";

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct RssConfig {
    pub feeds: Vec<String>,
}

#[derive(Clone)]
pub struct Feed {
    pub title: String,
    pub url: String,
}

#[derive(Clone)]
struct RssConfigHandlerData {
    config: RssConfig,
    videos: BTreeSet<Video>,
    feeds: Vec<Feed>,
}

pub struct RssConfigHandler {
    data: Arc<Mutex<RssConfigHandlerData>>,
    file_handler: Arc<tokio::sync::Mutex<ConfigFileHandler<RssConfig>>>,
}

impl RssConfigHandler {
    pub async fn add_feed(&self, url: &str) -> Result<(), ConfigError> {
        let (feed, videos) = Self::fetch_feed(url).await?;
        let new_config = {
            let mut data = self.data.lock();
            data.config.feeds.push(url.to_owned());
            data.feeds.push(feed);
            data.videos.extend(videos);

            data.config.clone()
        };

        self.save(&new_config).await
    }

    pub async fn import_youtube(&self, path: String) -> Result<(), ConfigError> {
        let content = fs::read_to_string(&path)
            .await
            .map_err(|_| ConfigError::ReadConfigFile)?;
        let mut urls: Vec<String> = content
            .trim()
            .split('\n')
            .skip(1)
            .map(|line| {
                let channel_id = line
                    .split(',')
                    .next()
                    .ok_or(ConfigError::ParseYoutubeTakeout)?;
                Ok(format!(
                    "https://www.youtube.com/feeds/videos.xml?channel_id={}",
                    channel_id
                ))
            })
            .collect::<Result<Vec<_>, ConfigError>>()?;

        let new_config = {
            let mut data = self.data.lock();
            data.config.feeds.append(&mut urls);
            data.config.clone()
        };

        self.save(&new_config).await
    }

    pub async fn remove_feed(&self, url: &str) -> Result<(), ConfigError> {
        let new_config = {
            let mut data = self.data.lock();
            data.config.feeds.retain(|feed| feed != url);
            data.feeds.retain(|feed| feed.url != url);

            data.config.clone()
        };

        self.save(&new_config).await
    }

    pub fn feeds(&self) -> Vec<Feed> {
        self.data.lock().feeds.clone()
    }

    async fn fetch_rss(url: &str) -> Result<atom_syndication::Feed, ConfigError> {
        let content = reqwest::get(url).await?.bytes().await?;
        Ok(atom_syndication::Feed::read_from(&content[..])?)
    }

    async fn fetch_feed(url: &str) -> Result<(Feed, Vec<Video>), ConfigError> {
        let rss = Self::fetch_rss(url).await?;
        let videos = Self::parse_videos(&rss).await?;
        let feed = Feed {
            title: rss.title().to_string(),
            url: url.to_owned(),
        };

        Ok((feed, videos))
    }

    async fn parse_videos(rss: &atom_syndication::Feed) -> Result<Vec<Video>, ConfigError> {
        use chrono::offset::Utc;

        let author = rss.title().as_str();
        let mut videos: Vec<Video> = rss
            .entries()
            .iter()
            .map(|entry| Self::parse_video(entry, author))
            .collect::<Result<_, _>>()?;

        let now = Utc::now();
        videos.retain(|video| now.years_since(video.date.into()).unwrap() < 1);
        Ok(videos)
    }

    fn parse_video(entry: &Entry, author: &str) -> Result<Video, ConfigError> {
        let description = entry
            .extensions()
            .get("media")
            .and_then(|media| media.get("group"))
            .and_then(|group| group.first())
            .and_then(|extension| extension.children().get("description"))
            .and_then(|description| description.first())
            .and_then(|description| description.value())
            .unwrap_or("")
            .to_string();

        let url = entry
            .links()
            .first()
            .ok_or(ConfigError::ParseVideo)?
            .href()
            .to_string();

        Ok(Video {
            title: entry.title().to_string(),
            url,
            author: author.to_string(),
            description,
            length: 0,
            date: entry.published().ok_or(ConfigError::ParseVideo)?.to_owned(),
        })
    }

    async fn save(&self, config: &RssConfig) -> Result<(), ConfigError> {
        let file_handler = self.file_handler.lock().await;
        file_handler.write(config).await
    }
}

#[async_trait]
impl Config for RssConfigHandler {
    async fn load() -> Result<Self, ConfigError> {
        let mut file_handler = ConfigFileHandler::from_config_file(CONFIG_NAME).await?;
        let config = file_handler.read().await?;

        let feeds = Vec::new();
        let videos = BTreeSet::new();

        Ok(Self {
            data: Arc::new(Mutex::new(RssConfigHandlerData {
                config,
                feeds,
                videos,
            })),
            file_handler: Arc::new(tokio::sync::Mutex::new(file_handler)),
        })
    }

    async fn fetch(&self) -> Result<(), ConfigError> {
        let feeds = {
            let mut data = self.data.lock();
            data.feeds.clear();
            data.videos.clear();
            data.config.feeds.clone()
        };

        let feed_data = future::join_all(feeds.iter().map(|url| Self::fetch_feed(url))).await;
        let mut data = self.data.lock();
        for result in feed_data {
            match result {
                Ok((feed, videos)) => {
                    data.feeds.push(feed);
                    data.videos.extend(videos);
                }
                Err(ConfigError::ReadFeed { .. }) => (),
                Err(err) => return Err(err),
            }
        }

        Ok(())
    }

    fn videos(&self) -> Vec<Video> {
        let data = self.data.lock();
        data.videos.clone().into_iter().rev().collect()
    }
}
