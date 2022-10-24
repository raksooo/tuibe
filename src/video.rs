use crate::config::error::FeedError;
use atom_syndication::Entry;
use chrono::{DateTime, FixedOffset};

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
            .map_err(|_| FeedError::ParseVideo)?
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
        })
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

        format!("{title:title_width$} {author:>author_width$} - {date} ")
    }
}
