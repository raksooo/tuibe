use super::{
    component::{Component, Frame},
    list::{List, Same},
    status_label::{StatusLabelActions, LOADING_STRING},
};
use crate::config::{
    common::CommonConfigHandler, config_message_channel::ConfigMessage, Config, Video,
};

use chrono::{DateTime, FixedOffset};
use crossterm::event::{Event, KeyCode};
use delegate::delegate;
use parking_lot::Mutex;
use std::{env, process::Stdio, sync::Arc};
use tokio::process::Command;
use tui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List as ListWidget, ListItem, Paragraph, Wrap},
};

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq)]
struct VideoListItem {
    video: Video,
    selected: bool,
}

impl From<VideoListItem> for ListItem<'static> {
    fn from(value: VideoListItem) -> Self {
        let selected = if value.selected { "✓" } else { " " };
        ListItem::new(format!(" {selected} {}", value.video.title))
    }
}

impl From<Video> for VideoListItem {
    fn from(value: Video) -> Self {
        VideoListItem::new(value, 0)
    }
}

impl Same for VideoListItem {
    fn same(&self, other: &Self) -> bool {
        self.video.url == other.video.url
    }
}

impl VideoListItem {
    pub fn new(video: Video, last_played_timestamp: i64) -> Self {
        let selected = video.date().timestamp() > last_played_timestamp;
        Self { video, selected }
    }

    pub fn toggle_selected(&mut self) {
        self.selected = !self.selected;
    }

    pub fn deselect(&mut self) {
        self.selected = !self.selected;
    }

    pub fn select_based_on_timestamp(&mut self, last_played_timestamp: i64) {
        self.selected = self.date().timestamp() > last_played_timestamp;
    }

    delegate! {
        to self.video {
            pub fn date(&self) -> DateTime<FixedOffset>;
        }
    }

    pub fn author(&self) -> String {
        self.video.author.clone()
    }

    pub fn description(&self) -> String {
        self.video.description.clone()
    }

    pub fn url(&self) -> String {
        self.video.url.clone()
    }
}

struct VideoList(List<VideoListItem>);

impl VideoList {
    pub fn new() -> Self {
        Self(List::new())
    }

    pub fn handle_config_message(
        &mut self,
        message: ConfigMessage<Video>,
        last_played_timestamp: i64,
    ) {
        match message {
            ConfigMessage::Clear => self.0.clear(),
            ConfigMessage::New(video) => {
                let video_list_item = VideoListItem::new(video, last_played_timestamp);
                self.0.add(video_list_item);
            }
            ConfigMessage::Remove(video) => self.0.remove(&video.into()),
            ConfigMessage::FinishedFetching => (), // Handled by FeedView
            ConfigMessage::Error(_) => (),         // Handled by FeedView
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

pub struct FeedView {
    actions: StatusLabelActions,
    common_config: Arc<CommonConfigHandler>,
    loading_id: Arc<Mutex<Option<usize>>>,
    video_list: Arc<Mutex<VideoList>>,
}

impl FeedView {
    pub fn new(
        actions: StatusLabelActions,
        common_config: CommonConfigHandler,
        config: Arc<impl Config + Send + Sync + 'static>,
    ) -> Self {
        let feed_view = Self {
            actions,
            common_config: Arc::new(common_config),
            loading_id: Default::default(),
            video_list: Arc::new(Mutex::new(VideoList::new())),
        };

        feed_view.listen_config_messages(config);
        feed_view
    }

    fn listen_config_messages(&self, config: Arc<impl Config + Send + Sync + 'static>) {
        let loading_id = self.loading_id.clone();
        let actions = self.actions.clone();
        let common_config = self.common_config.clone();
        let video_list = self.video_list.clone();
        tokio::spawn(async move {
            {
                let mut loading_id = loading_id.lock();
                *loading_id = Some(actions.start_status(LOADING_STRING));
            }

            let mut receiver = config.subscribe();
            while let Some(message) = receiver.recv().await {
                Self::handle_config_message(
                    message,
                    loading_id.clone(),
                    actions.clone(),
                    common_config.clone(),
                    video_list.clone(),
                )
                .await;
            }
        });
    }

    async fn handle_config_message(
        message: ConfigMessage<Video>,
        loading_id: Arc<Mutex<Option<usize>>>,
        actions: StatusLabelActions,
        common_config: Arc<CommonConfigHandler>,
        video_list: Arc<Mutex<VideoList>>,
    ) {
        match message {
            ConfigMessage::Error(error) => actions.handle_error_async(error, true).await,
            ConfigMessage::FinishedFetching => {
                let mut loading_id = loading_id.lock();
                if let Some(loading_id) = *loading_id {
                    actions.finish_status(loading_id);
                }
                *loading_id = None;
            }
            _ => {
                {
                    let mut video_list = video_list.lock();
                    video_list.handle_config_message(
                        message,
                        common_config.clone().last_played_timestamp(),
                    );
                }
                actions.redraw_async().await;
            }
        }
    }

    fn set_current_as_last_played(&mut self) {
        let Some(last_played_timestamp) = self.video_list.lock().current_timestamp() else { return };
        self.update_last_played_timestamp(last_played_timestamp);
    }

    fn update_last_played_timestamp(&mut self, last_played_timestamp: i64) {
        {
            self.video_list
                .lock()
                .update_last_played_timestamp(last_played_timestamp);
        }

        let common_config = self.common_config.clone();
        let actions = self.actions.clone();
        tokio::spawn(async move {
            actions
                .redraw_or_error_async(
                    common_config
                        .set_last_played_timestamp(last_played_timestamp)
                        .await,
                    true,
                )
                .await;
        });
    }

    fn play(&mut self) {
        let selected_videos = self.video_list.lock().selected_videos();

        if let Some(newest_video) = selected_videos.first() {
            let finish_status = self.actions.show_label("Playing...");
            self.update_last_played_timestamp(newest_video.date().timestamp());

            let player = self.get_player();
            let actions = self.actions.clone();
            tokio::spawn(async move {
                let videos = selected_videos.iter().map(|video| video.url());
                let play_result = Command::new(player)
                    .args(videos)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .await;

                finish_status();
                actions.redraw_or_error_async(play_result, true).await;
            });
        }
    }

    fn get_player(&self) -> String {
        env::args()
            .skip_while(|arg| arg != "--player")
            .nth(1)
            .unwrap_or_else(|| self.common_config.player())
    }
}

impl Component for FeedView {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        let description_height = 10;
        let description_y = area.height - description_height;
        let list_area = Rect::new(area.x, 0, area.width, description_y - 1);
        let description_area = Rect::new(area.x, description_y, area.width, description_height);

        let video_list = self.video_list.lock();
        let description = video_list.current_description();

        let metadata_width = 35;
        let title_area = Rect::new(
            list_area.x,
            list_area.y,
            list_area.width - metadata_width - 3,
            list_area.height,
        );
        let metadata_area = Rect::new(
            list_area.x + list_area.width - metadata_width,
            list_area.y + 1,
            metadata_width,
            list_area.height - 1,
        );

        let list = video_list.list(area.height.into());
        let styled_list = list
            .block(Block::default().title("Videos"))
            .style(Style::default().fg(Color::White));

        let metadata_list = video_list.metadata_list(area.height.into());
        let styled_metadata_list = metadata_list
            .block(Block::default())
            .style(Style::default().fg(Color::White));

        f.render_widget(styled_list, title_area);
        f.render_widget(styled_metadata_list, metadata_area);
        f.render_widget(description, description_area);
    }

    fn handle_event(&mut self, event: Event) {
        if let Event::Key(event) = event {
            match event.code {
                KeyCode::Up => self.video_list.lock().move_up(),
                KeyCode::Down => self.video_list.lock().move_down(),
                KeyCode::Char('j') => self.video_list.lock().move_down(),
                KeyCode::Char('k') => self.video_list.lock().move_up(),
                KeyCode::Char('g') => self.video_list.lock().move_top(),
                KeyCode::Char('G') => self.video_list.lock().move_bottom(),
                KeyCode::Char('a') => self.video_list.lock().deselect_all(),
                KeyCode::Char(' ') => self.video_list.lock().toggle_current(),
                KeyCode::Char('p') => self.play(),
                KeyCode::Char('n') => self.set_current_as_last_played(),
                _ => return,
            }
        }

        self.actions.redraw();
    }

    fn registered_events(&self) -> Vec<(String, String)> {
        vec![
            (String::from("j"), String::from("Down")),
            (String::from("k"), String::from("Up")),
            (String::from("g"), String::from("Top")),
            (String::from("G"), String::from("Bottom")),
            (String::from("Space"), String::from("Select")),
            (String::from("p"), String::from("Play")),
            (String::from("n"), String::from("Update last played")),
            (String::from("a"), String::from("Deselect all")),
        ]
    }
}
