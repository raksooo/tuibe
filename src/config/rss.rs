use super::file_handler::ConfigFileHandler;
use crate::{
    config::{
        config::{Config, ConfigData, ConfigUpdate},
        error::{ConfigError, FeedError},
    },
    video::Video,
};
use async_trait::async_trait;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashMap},
    future::Future,
    sync::Arc,
};
use tokio::sync::oneshot;

const CONFIG_NAME: &str = "rss";

#[derive(Clone, Serialize, Deserialize)]
pub struct RssConfig {
    pub channel_ids: Vec<String>,
}

impl Default for RssConfig {
    fn default() -> Self {
        Self {
            channel_ids: Vec::new(),
        }
    }
}

pub struct RssConfigHandler {
    config: Arc<Mutex<RssConfig>>,
    data: Arc<Mutex<ConfigData>>,
    file_handler: Arc<tokio::sync::Mutex<ConfigFileHandler>>,
}

impl RssConfigHandler {
    async fn parse_videos(rss: &atom_syndication::Feed) -> Result<Vec<Video>, FeedError> {
        let author = rss.title().as_str();
        rss.entries()
            .iter()
            .map(|entry| Video::from_rss_entry(entry, author))
            .collect()
    }

    async fn fetch_rss(url: &str) -> Result<atom_syndication::Feed, FeedError> {
        let content = reqwest::get(url)
            .await
            .map_err(|_| FeedError::FetchFeed)?
            .bytes()
            .await
            .map_err(|_| FeedError::FetchFeed)?;

        atom_syndication::Feed::read_from(&content[..])
            .map_err(|error| FeedError::ReadFeed { error })
    }

    async fn fetch_channel(id: &str, data: &mut ConfigData) -> Result<(), ConfigError> {
        let url = format!("https://www.youtube.com/feeds/videos.xml?channel_id={}", id);

        let rss = Self::fetch_rss(&url).await?;
        let videos = Self::parse_videos(&rss).await?;

        data.channels
            .insert(id.to_string(), rss.title().to_string());
        for video in videos {
            data.videos.insert(video);
        }

        Ok(())
    }

    fn modify<R, F>(&self, f: F) -> oneshot::Receiver<ConfigUpdate>
    where
        R: Future<Output = Result<(RssConfig, ConfigData), ConfigError>> + Send,
        F: FnOnce(RssConfig, ConfigData) -> R + Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        let config = Arc::clone(&self.config);
        let data = Arc::clone(&self.data);
        let file_handler = Arc::clone(&self.file_handler);

        tokio::spawn(async move {
            let new_data = Self::modify_impl(config, data, file_handler, f).await;
            let _ = tx.send(new_data);
        });

        rx
    }

    async fn modify_impl<R, F>(
        config: Arc<Mutex<RssConfig>>,
        data: Arc<Mutex<ConfigData>>,
        file_handler: Arc<tokio::sync::Mutex<ConfigFileHandler>>,
        f: F,
    ) -> ConfigUpdate
    where
        R: Future<Output = Result<(RssConfig, ConfigData), ConfigError>> + Send,
        F: FnOnce(RssConfig, ConfigData) -> R + Send,
    {
        let (old_config, old_data) = {
            let config = config.lock();
            let data = data.lock();
            (config.clone(), data.clone())
        };

        let (new_config, new_data) = f(old_config, old_data).await?;
        {
            let mut config = config.lock();
            let mut data = data.lock();
            *config = new_config.clone();
            *data = new_data.clone();
        };

        let file_handler = file_handler.lock().await;
        file_handler.write(&new_config).await?;

        Ok(new_data)
    }
}

#[async_trait]
impl Config for RssConfigHandler {
    async fn load() -> Result<Self, ConfigError> {
        let mut file_handler = ConfigFileHandler::from_config_file(CONFIG_NAME).await?;
        let config = file_handler.read().await?;

        let channels: HashMap<String, String> = HashMap::new();
        let videos: BTreeSet<Video> = BTreeSet::new();

        Ok(Self {
            config: Arc::new(Mutex::new(config)),
            data: Arc::new(Mutex::new(ConfigData { channels, videos })),
            file_handler: Arc::new(tokio::sync::Mutex::new(file_handler)),
        })
    }

    fn fetch(&self) -> oneshot::Receiver<ConfigUpdate> {
        {
            let mut data = self.data.lock();
            data.channels.clear();
            data.videos.clear();
        }

        self.modify(|config, mut data| async {
            for channel_id in config.channel_ids.iter() {
                Self::fetch_channel(channel_id, &mut data).await?;
            }

            Ok((config, data))
        })
    }

    fn add_channel(&self, id: String) -> oneshot::Receiver<ConfigUpdate> {
        self.modify(|mut config, mut data| async move {
            Self::fetch_channel(&id, &mut data).await?;
            config.channel_ids.push(id);
            Ok((config, data))
        })
    }

    fn remove_subscription(&self, id: String) -> oneshot::Receiver<ConfigUpdate> {
        // TODO: Remove videos
        self.modify(|mut config, mut data| async move {
            config
                .channel_ids
                .retain(|channel| channel.to_owned() != id);
            data.channels.remove(&id);
            Ok((config, data))
        })
    }

    fn data(&self) -> ConfigData {
        let data = self.data.lock();
        data.clone()
    }
}
