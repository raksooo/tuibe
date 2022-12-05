use super::{
    channel::{BackendMessage, BackendReceiver, BackendSender},
    Backend, Video,
};
use crate::{config_error::ConfigError, file_handler::ConfigFileHandler};

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
struct RssBackendData {
    videos: Vec<Video>,
    feeds: Vec<Feed>,
}

#[derive(Clone)]
struct RssBackendInner {
    config: RssConfig,
    data: Option<RssBackendData>,
}

pub struct RssBackend {
    inner: Arc<Mutex<RssBackendInner>>,
    file_handler: tokio::sync::Mutex<ConfigFileHandler<RssConfig>>,
    video_sender: Arc<BackendSender<Video>>,
    feed_sender: Arc<BackendSender<Feed>>,
}

impl RssBackend {
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
        let rss_backend = {
            let mut inner = self.inner.lock();
            inner.config.feeds.push(url.to_owned());
            inner.config.clone()
        };

        self.save(&rss_backend).await
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

        let config = {
            let mut inner = self.inner.lock();
            inner.config.feeds.append(&mut urls);
            inner.config.clone()
        };

        self.save(&config).await
    }

    pub async fn remove_feed(&self, url: &str) -> Result<(), ConfigError> {
        let new_config = {
            let mut inner = self.inner.lock();
            inner.config.feeds.retain(|feed| feed != url);

            if let Some(ref mut data) = inner.data {
                data.feeds.retain(|feed| {
                    let keep = feed.url != url;
                    if !keep {
                        self.feed_sender.send(BackendMessage::Remove(feed.clone()));
                    }
                    keep
                });
                data.videos.retain(|video| {
                    let keep = video.feed_url != url;
                    if !keep {
                        self.video_sender
                            .send(BackendMessage::Remove(video.clone()));
                    }
                    keep
                });
            }

            inner.config.clone()
        };

        self.save(&new_config).await
    }

    pub fn subscribe_feeds(&self) -> BackendReceiver<Feed> {
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
        inner: Arc<Mutex<RssBackendInner>>,
        video_sender: Arc<BackendSender<Video>>,
        feed_sender: Arc<BackendSender<Feed>>,
    ) -> Result<(), ConfigError> {
        let rss = Self::fetch_rss(url).await?;
        Self::parse_videos(&rss, url, inner.clone(), video_sender.clone()).await?;

        let mut inner = inner.lock();
        let feed = Feed {
            title: rss.title().to_string(),
            url: url.to_owned(),
        };

        feed_sender.send(BackendMessage::New(feed.clone()));
        if let Some(ref mut data) = inner.data {
            data.feeds.push(feed);
        }

        video_sender.send(BackendMessage::FinishedFetching);
        feed_sender.send(BackendMessage::FinishedFetching);

        Ok(())
    }

    async fn parse_videos(
        rss: &atom_syndication::Feed,
        feed_url: &str,
        inner: Arc<Mutex<RssBackendInner>>,
        video_sender: Arc<BackendSender<Video>>,
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
        inner: Arc<Mutex<RssBackendInner>>,
        video_sender: Arc<BackendSender<Video>>,
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

        video_sender.send(BackendMessage::New(video.clone()));
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
                    Err(error) => video_sender.send(BackendMessage::Error(error.to_string())),
                }
            });
        }
    }
}

#[async_trait]
impl Backend for RssBackend {
    async fn load() -> Result<Self, ConfigError> {
        let mut file_handler = ConfigFileHandler::from_config_file(CONFIG_NAME).await?;
        let config = file_handler.read().await?;

        let inner = RssBackendInner { config, data: None };

        Ok(Self {
            inner: Arc::new(Mutex::new(inner)),
            file_handler: tokio::sync::Mutex::new(file_handler),
            video_sender: Arc::new(BackendSender::new()),
            feed_sender: Arc::new(BackendSender::new()),
        })
    }

    fn subscribe(&self) -> BackendReceiver<Video> {
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
            self.video_sender.send(BackendMessage::Clear);
            self.feed_sender.send(BackendMessage::Clear);
        }
        self.fetch();
    }
}
