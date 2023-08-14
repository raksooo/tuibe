use super::list::{List, Same};
use crate::backend::{channel::BackendMessage, Video};

use chrono::{DateTime, FixedOffset};
use delegate::delegate;
use std::cmp::Reverse;
use tui::{
    style::{Color, Style},
    widgets::{Block, Borders, List as ListWidget, ListItem, Paragraph, Wrap},
};

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VideoListItem {
    video: Reverse<Video>,
    selected: bool,
}

impl From<VideoListItem> for ListItem<'static> {
    fn from(value: VideoListItem) -> Self {
        let selected = if value.selected { "✓" } else { " " };
        ListItem::new(format!(" {selected} {}", value.video.0.title))
    }
}

impl From<Video> for VideoListItem {
    fn from(value: Video) -> Self {
        VideoListItem::new(value, 0)
    }
}

impl Same for VideoListItem {
    fn same(&self, other: &Self) -> bool {
        self.video.0.url == other.video.0.url
    }
}

impl VideoListItem {
    pub fn new(video: Video, last_played_timestamp: i64) -> Self {
        let selected = video.date.timestamp() > last_played_timestamp;
        Self {
            video: Reverse(video),
            selected,
        }
    }

    pub fn toggle_selected(&mut self) {
        self.selected = !self.selected;
    }

    pub fn deselect(&mut self) {
        self.selected = false;
    }

    pub fn select_based_on_timestamp(&mut self, last_played_timestamp: i64) {
        self.selected = self.date().timestamp() > last_played_timestamp;
    }

    pub fn date(&self) -> DateTime<FixedOffset> {
        self.video.0.date
    }

    pub fn author(&self) -> String {
        self.video.0.author.clone()
    }

    pub fn description(&self) -> String {
        self.video.0.description.clone()
    }

    pub fn url(&self) -> String {
        self.video.0.url.clone()
    }
}

pub struct VideoList(List<VideoListItem>);

impl VideoList {
    pub fn new() -> Self {
        Self(List::new())
    }

    pub fn handle_backend_message(
        &mut self,
        message: BackendMessage<Video>,
        last_played_timestamp: i64,
    ) {
        match message {
            BackendMessage::Clear => self.0.clear(),
            BackendMessage::New(video) => {
                let video_list_item = VideoListItem::new(video, last_played_timestamp);
                self.0.add(video_list_item);
            }
            BackendMessage::Remove(video) => self.0.remove(&video.into()),
            BackendMessage::FinishedFetching => (), // Handled by FeedView
            BackendMessage::Error(_) => (),         // Handled by FeedView
        }
    }

    delegate! {
        to self.0 {
            pub fn move_up(&mut self);
            pub fn move_down(&mut self);
            pub fn move_top(&mut self);
            pub fn move_bottom(&mut self);
            pub fn list(&self, height: usize) -> ListWidget<'_>;
        }
    }

    pub fn metadata_list(&self, height: usize) -> ListWidget<'_> {
        self.0.map_list(height, |video| {
            let author_width = 15;
            let author = video.author();
            let author = author.get(..author_width).unwrap_or(&author);
            let date = video.date().format("%Y-%m-%d %H:%M");

            ListItem::new(format!("{author:>author_width$} - {date} "))
        })
    }

    pub fn toggle_current(&mut self) {
        self.0.mutate_current_item(|video| video.toggle_selected());
    }

    pub fn deselect_all(&mut self) {
        self.0.mutate_every_item(|video| video.deselect());
    }

    pub fn current_timestamp(&self) -> Option<i64> {
        self.0
            .get_current_item()
            .map(|item| item.date().timestamp())
    }

    pub fn update_last_played_timestamp(&mut self, last_played_timestamp: i64) {
        self.0
            .mutate_every_item(|video| video.select_based_on_timestamp(last_played_timestamp));
    }

    pub fn selected_videos(&self) -> Vec<VideoListItem> {
        self.0
            .iter()
            .cloned()
            .filter_map(|video| video.selected.then_some(video))
            .collect()
    }

    pub fn current_video(&self) -> Option<VideoListItem> {
        self.0.get_current_item()
    }

    pub fn current_description(&self) -> Paragraph<'_> {
        let description = self
            .0
            .get_current_item()
            .map(|video| video.description())
            .unwrap_or_default();
        Paragraph::new(description)
            .block(Block::default().title("Description").borders(Borders::TOP))
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true })
    }
}
