use super::file_handler::ConfigFileHandler;
use crate::config::{
    config::{Config, ConfigResult, Video},
    error::ConfigError,
};
use async_trait::async_trait;
use atom_syndication::Entry;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, future::Future, sync::Arc};
use tokio::{fs, sync::oneshot};

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
    file_handler: Arc<tokio::sync::Mutex<ConfigFileHandler>>,
}

impl RssConfigHandler {
    pub fn add_feed(&self, url: &str) -> oneshot::Receiver<ConfigResult> {
        let url = String::from(url);
        self.modify(|mut data| async move {
            Self::fetch_feed(&url, &mut data).await?;
            data.config.feeds.push(url);
            Ok(data)
        })
    }

    pub async fn import_youtube(&self, path: String) {
        println!("Importing subscriptions...");
        let content = fs::read_to_string(&path)
            .await
            .expect("Failed to read file");
        let mut urls: Vec<String> = content
            .trim()
            .split('\n')
            .skip(1)
            .map(|line| {
                let channel_id = line.split(',').next().expect("Import failed");
                format!(
                    "https://www.youtube.com/feeds/videos.xml?channel_id={}",
                    channel_id
                )
            })
            .collect();

        let new_config = {
            let mut data = self.data.lock();
            data.config.feeds.append(&mut urls);
            data.config.clone()
        };

        let file_handler = self.file_handler.lock().await;
        file_handler
            .write(&new_config)
            .await
            .expect("Failed to write to config file");

        println!("Done.");
    }

    pub fn remove_feed(&self, url: &str) -> oneshot::Receiver<ConfigResult> {
        let url = String::from(url);
        self.modify(|mut data| async move {
            data.config.feeds.retain(|feed| feed != &url);
            data.feeds.retain(|feed| feed.url != url);
            Ok(data)
        })
    }

    pub fn feeds(&self) -> Vec<Feed> {
        let data = self.data.lock();
        data.feeds.clone()
    }

    async fn fetch_rss(url: &str) -> Result<atom_syndication::Feed, ConfigError> {
        let content = reqwest::get(url)
            .await
            .map_err(|_| ConfigError::FetchFeed)?
            .bytes()
            .await
            .map_err(|_| ConfigError::FetchFeed)?;

        atom_syndication::Feed::read_from(&content[..])
            .map_err(|error| ConfigError::ReadFeed { error })
    }

    async fn fetch_feed(url: &str, data: &mut RssConfigHandlerData) -> Result<(), ConfigError> {
        let rss = Self::fetch_rss(url).await?;
        let videos = Self::parse_videos(&rss).await?;

        data.feeds.push(Feed {
            title: rss.title().to_string(),
            url: String::from(url),
        });
        data.videos.extend(videos);

        Ok(())
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
            .ok_or("")
            .map_err(|_| ConfigError::ParseVideo)?
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

    fn modify<R, F>(&self, f: F) -> oneshot::Receiver<ConfigResult>
    where
        R: Future<Output = Result<RssConfigHandlerData, ConfigError>> + Send,
        F: FnOnce(RssConfigHandlerData) -> R + Send + 'static,
    {
        let (sender, receiver) = oneshot::channel();
        let data = Arc::clone(&self.data);
        let file_handler = Arc::clone(&self.file_handler);

        tokio::spawn(async move {
            let new_data = Self::modify_impl(data, file_handler, f).await;
            let _ = sender.send(new_data);
        });

        receiver
    }

    async fn modify_impl<R, F>(
        data: Arc<Mutex<RssConfigHandlerData>>,
        file_handler: Arc<tokio::sync::Mutex<ConfigFileHandler>>,
        f: F,
    ) -> ConfigResult
    where
        R: Future<Output = Result<RssConfigHandlerData, ConfigError>> + Send,
        F: FnOnce(RssConfigHandlerData) -> R + Send,
    {
        let old_data = {
            let data = data.lock();
            data.clone()
        };

        let new_data = f(old_data).await?;
        {
            let mut data = data.lock();
            *data = new_data.clone();
        };

        let file_handler = file_handler.lock().await;
        file_handler.write(&new_data.config).await?;

        Ok(new_data.videos.into_iter().collect())
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

    fn fetch(&self) -> oneshot::Receiver<ConfigResult> {
        {
            let mut data = self.data.lock();
            data.feeds.clear();
            data.videos.clear();
        }

        self.modify(|mut data| async {
            for url in data.config.clone().feeds.iter() {
                match Self::fetch_feed(url, &mut data).await {
                    Ok(_) | Err(ConfigError::ReadFeed { .. }) => (),
                    Err(error) => return Err(error),
                };
            }

            Ok(data)
        })
    }

    fn videos(&self) -> Vec<Video> {
        let data = self.data.lock();
        data.videos.clone().into_iter().rev().collect()
    }
}
