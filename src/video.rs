use crate::error::FeedError;
use atom_syndication::Entry;
use chrono::{DateTime, FixedOffset};
use std::cmp::Ordering;

#[derive(Clone, Eq, PartialEq)]
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

impl Ord for Video {
    fn cmp(&self, other: &Self) -> Ordering {
        self.date.cmp(&other.date)
    }
}

impl PartialOrd for Video {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
