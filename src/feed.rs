use crate::{config::Config, error::FeedError};
use atom_syndication::Entry;
use chrono::{DateTime, FixedOffset};

pub struct Video {
    pub title: String,
    pub url: String,
    pub author: String,
    pub description: String,
    pub length: u32,
    pub date: DateTime<FixedOffset>,
    pub selected: bool,
}

impl Video {
    pub fn from_rss_entry(entry: &Entry, author: &str) -> Result<Video, FeedError> {
        let description = entry
            .extensions()
            .get("media")
            .and_then(|media| media.get("group"))
            .and_then(|group| group.first())
            .and_then(|extension| extension.children().get("description"))
            .and_then(|description| description.first())
            .and_then(|description| description.value())
            .ok_or("")
            .map_err(|_| FeedError::ParseVideoDescription)?
            .to_string();

        let url = entry
            .links()
            .first()
            .ok_or(FeedError::ParseVideo)?
            .href()
            .to_string();

        Ok(Video {
            title: entry.title().as_str().to_string(),
            url,
            author: author.to_string(),
            description,
            length: 0,
            date: entry.published().ok_or(FeedError::ParseVideo)?.to_owned(),
            selected: false,
        })
    }

    pub fn toggle_selected(&mut self) {
        self.selected = !self.selected;
    }

    pub fn select_if_newer(&mut self, timestamp: i64) {
        if self.date.timestamp() > timestamp {
            self.selected = true;
        }
    }

    pub fn get_label(&self, width: usize) -> String {
        // Subtract width of datetime and horizontal padding and checkmark.
        let width = width - 16 - 2 - 2;
        // Split the area between title and author 3/4 for author.
        let title_width = 3 * width / 4;
        let author_width = width - title_width;

        let date = self.date.format("%Y-%m-%d %H:%M");
        let title = self.title.get(..title_width).unwrap_or(&self.title);
        let author = self.author.get(..author_width).unwrap_or(&self.author);

        let checkmark = if self.selected { "✓" } else { " " };

        format!(" {checkmark} {title:title_width$} {author:>author_width$} - {date} ")
    }
}

pub struct Feed {
    pub videos: Vec<Video>,
}

impl Feed {
    pub async fn from_config(config: &Config) -> Result<Feed, FeedError> {
        let mut videos: Vec<Video> = Vec::new();
        for channel_url in config.subscriptions.iter() {
            let channel_videos = Self::fetch_videos(channel_url).await?;
            for mut video in channel_videos {
                video.select_if_newer(config.last_played_timestamp);
                videos.push(video);
            }
        }

        videos.sort_by(|a, b| b.date.partial_cmp(&a.date).unwrap());

        Ok(Feed { videos })
    }

    async fn fetch_videos(url: &String) -> Result<Vec<Video>, FeedError> {
        let feed = Self::fetch_rss(url).await?;
        let author = feed.title().as_str();

        feed.entries()
            .iter()
            .map(|entry| Video::from_rss_entry(entry, author))
            .collect()
    }

    async fn fetch_rss(url: &String) -> Result<atom_syndication::Feed, FeedError> {
        let content = reqwest::get(url)
            .await
            .map_err(|_| FeedError::FetchFeed)?
            .bytes()
            .await
            .map_err(|_| FeedError::FetchFeed)?;

        atom_syndication::Feed::read_from(&content[..])
            .map_err(|error| FeedError::ReadFeed { error })
    }
}
