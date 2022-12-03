use super::{
    config_message_channel::{ConfigMessage, ConfigReceiver, ConfigSender},
    file_handler::ConfigFileHandler,
};
use crate::config::{error::ConfigError, Config, Video};

use async_trait::async_trait;
use atom_syndication::Entry;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{cmp::Reverse, sync::Arc};
use tokio::fs;

const CONFIG_NAME: &str = "rss";

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct RssConfig {
    pub feeds: Vec<String>,
}

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct Feed {
    pub title: String,
    pub url: String,
}

#[derive(Clone, Default)]
struct RssConfigHandlerData {
    videos: Vec<Video>,
    feeds: Vec<Feed>,
}

#[derive(Clone)]
struct RssConfigHandlerInner {
    config: RssConfig,
    data: Option<RssConfigHandlerData>,
}

pub struct RssConfigHandler {
    inner: Arc<Mutex<RssConfigHandlerInner>>,
    file_handler: tokio::sync::Mutex<ConfigFileHandler<RssConfig>>,
    video_sender: Arc<ConfigSender<Video>>,
    feed_sender: Arc<ConfigSender<Feed>>,
}

impl RssConfigHandler {
    pub async fn add_feed(&self, url: &str) -> Result<(), ConfigError> {
        {
            let inner = self.inner.lock();
            if inner.config.feeds.contains(&url.to_string()) {
                return Ok(());
            }
        }

        Self::fetch_feed(
            url,
            self.inner.clone(),
            self.video_sender.clone(),
            self.feed_sender.clone(),
        )
        .await?;
        let new_config = {
            let mut inner = self.inner.lock();
            inner.config.feeds.push(url.to_owned());
            inner.config.clone()
        };

        self.save(&new_config).await
    }

    pub async fn import_youtube(&self, path: &str) -> Result<(), ConfigError> {
        let content = fs::read_to_string(&path)
            .await
            .map_err(ConfigError::ReadYoutubeTakeout)?;
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
            let mut inner = self.inner.lock();
            inner.config.feeds.append(&mut urls);
            inner.config.clone()
        };

        self.save(&new_config).await
    }

    pub async fn remove_feed(&self, url: &str) -> Result<(), ConfigError> {
        let new_config = {
            let mut inner = self.inner.lock();
            inner.config.feeds.retain(|feed| feed != url);

            if let Some(ref mut data) = inner.data {
                data.feeds.retain(|feed| {
                    let keep = feed.url != url;
                    if !keep {
                        self.feed_sender.send(ConfigMessage::Remove(feed.clone()));
                    }
                    keep
                });
                data.videos.retain(|video| {
                    let keep = video.feed_url != url;
                    if !keep {
                        self.video_sender.send(ConfigMessage::Remove(video.clone()));
                    }
                    keep
                });
            }

            inner.config.clone()
        };

        self.save(&new_config).await
    }

    pub fn subscribe_feeds(&self) -> ConfigReceiver<Feed> {
        let inner = self.inner.lock();
        let feeds = inner
            .data
            .as_ref()
            .map(|data| data.feeds.clone())
            .unwrap_or_default();
        self.feed_sender.subscribe(feeds)
    }

    async fn fetch_rss(url: &str) -> Result<atom_syndication::Feed, ConfigError> {
        let content = reqwest::get(url).await?.bytes().await?;
        Ok(atom_syndication::Feed::read_from(&content[..])?)
    }

    async fn fetch_feed(
        url: &str,
        inner: Arc<Mutex<RssConfigHandlerInner>>,
        video_sender: Arc<ConfigSender<Video>>,
        feed_sender: Arc<ConfigSender<Feed>>,
    ) -> Result<(), ConfigError> {
        let rss = Self::fetch_rss(url).await?;
        Self::parse_videos(&rss, url, inner.clone(), video_sender.clone()).await?;

        let mut inner = inner.lock();
        let feed = Feed {
            title: rss.title().to_string(),
            url: url.to_owned(),
        };

        feed_sender.send(ConfigMessage::New(feed.clone()));
        if let Some(ref mut data) = inner.data {
            data.feeds.push(feed);
        }

        video_sender.send(ConfigMessage::FinishedFetching);
        feed_sender.send(ConfigMessage::FinishedFetching);

        Ok(())
    }

    async fn parse_videos(
        rss: &atom_syndication::Feed,
        feed_url: &str,
        inner: Arc<Mutex<RssConfigHandlerInner>>,
        video_sender: Arc<ConfigSender<Video>>,
    ) -> Result<(), ConfigError> {
        let author = rss.title().as_str();
        rss.entries().iter().try_for_each(|entry| {
            Self::parse_video(entry, author, feed_url, inner.clone(), video_sender.clone())
        })
    }

    fn parse_video(
        entry: &Entry,
        author: &str,
        feed_url: &str,
        inner: Arc<Mutex<RssConfigHandlerInner>>,
        video_sender: Arc<ConfigSender<Video>>,
    ) -> Result<(), ConfigError> {
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

        let date = entry.published().ok_or(ConfigError::ParseVideo)?.to_owned();

        let video = Video {
            title: entry.title().to_string(),
            url,
            author: author.to_string(),
            feed_url: feed_url.to_string(),
            description,
            length: 0,
            date: Reverse(date),
        };

        video_sender.send(ConfigMessage::New(video.clone()));
        if let Some(ref mut data) = inner.lock().data {
            data.videos.push(video);
        }

        Ok(())
    }

    async fn save(&self, config: &RssConfig) -> Result<(), ConfigError> {
        let file_handler = self.file_handler.lock().await;
        file_handler.write(config).await
    }

    fn fetch(&self) {
        let mut inner = self.inner.lock();
        if inner.data.is_none() {
            inner.data = Some(Default::default());
        }

        for url in inner.config.feeds.iter() {
            let url = url.clone();
            let inner = self.inner.clone();
            let video_sender = self.video_sender.clone();
            let feed_sender = self.feed_sender.clone();
            tokio::spawn(async move {
                match Self::fetch_feed(&url, inner, video_sender.clone(), feed_sender.clone()).await
                {
                    Ok(()) => (),
                    Err(ConfigError::ReadFeed { .. }) => (),
                    Err(error) => video_sender.send(ConfigMessage::Error(error.to_string())),
                }
            });
        }
    }
}

#[async_trait]
impl Config for RssConfigHandler {
    async fn load() -> Result<Self, ConfigError> {
        let mut file_handler = ConfigFileHandler::from_config_file(CONFIG_NAME).await?;
        let config = file_handler.read().await?;

        let inner = RssConfigHandlerInner { config, data: None };

        Ok(Self {
            inner: Arc::new(Mutex::new(inner)),
            file_handler: tokio::sync::Mutex::new(file_handler),
            video_sender: Arc::new(ConfigSender::new()),
            feed_sender: Arc::new(ConfigSender::new()),
        })
    }

    fn subscribe(&self) -> ConfigReceiver<Video> {
        let videos = {
            let inner = self.inner.lock();
            inner
                .data
                .as_ref()
                .map(|data| data.videos.clone())
                .unwrap_or_default()
        };
        let receiver = self.video_sender.subscribe(videos);

        self.fetch();
        receiver
    }

    fn refetch(&self) {
        {
            let mut inner = self.inner.lock();
            if let Some(ref mut data) = inner.data {
                data.feeds.clear();
                data.videos.clear();
            }
            self.video_sender.send(ConfigMessage::Clear);
            self.feed_sender.send(ConfigMessage::Clear);
        }
        self.fetch();
    }
}
