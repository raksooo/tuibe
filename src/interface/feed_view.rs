use super::{
    component::{Component, Frame},
    dialog::Dialog,
    error_handler::ErrorHandlerActions,
    list::generate_items,
};
use crate::config::{
    common::CommonConfigHandler, config_message_channel::ConfigMessage, Config, Video,
};

use crossterm::event::{Event, KeyCode, KeyEvent};
use parking_lot::Mutex;
use sorted_vec::SortedSet;
use std::{env, process::Stdio, sync::Arc};
use tokio::process::Command;
use tui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, Paragraph, Wrap},
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct VideoListItem {
    video: Video,
    pub selected: bool,
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
        self.selected = self.timestamp() > last_played_timestamp;
    }

    pub fn label(&self, width: usize) -> String {
        self.video.label(width)
    }

    pub fn description(&self) -> String {
        self.video.description.clone()
    }

    pub fn timestamp(&self) -> i64 {
        self.video.date().timestamp()
    }

    pub fn url(&self) -> String {
        self.video.url.clone()
    }
}

struct VideoListInner {
    videos: SortedSet<VideoListItem>,
    current_index: Option<usize>,
}

struct VideoList(Mutex<VideoListInner>);

impl VideoList {
    pub fn new() -> Self {
        Self(Mutex::new(VideoListInner {
            videos: SortedSet::new(),
            current_index: None,
        }))
    }

    pub fn handle_config_message(&self, message: ConfigMessage, last_played_timestamp: i64) {
        match message {
            ConfigMessage::Clear => {
                let mut inner = self.0.lock();
                inner.current_index = None;
                inner.videos.clear();
            }
            ConfigMessage::NewVideo(video) => {
                let mut inner = self.0.lock();
                inner.current_index = inner.current_index.or(Some(0));
                let video_list_item = VideoListItem::new(video, last_played_timestamp);
                inner
                    .videos
                    .mutate_vec(|videos| videos.push(video_list_item));
            }
            ConfigMessage::RemoveVideos(_) => todo!(),
            ConfigMessage::Error(_) => (), // Handled by FeedView
        }
    }

    pub fn move_up(&self) {
        self.mutate_current_index(|current_index| current_index.saturating_sub(1));
    }

    pub fn move_down(&self) {
        self.mutate_current_index(|current_index| current_index.saturating_add(1));
    }

    pub fn move_top(&self) {
        self.mutate_current_index(|_| 0);
    }

    pub fn move_bottom(&self) {
        self.mutate_current_index(|_| usize::MAX);
    }

    pub fn toggle_current(&self) {
        self.mutate_current_video(|video| video.toggle_selected());
    }

    pub fn deselect_all(&self) {
        self.mutate_every_video(|video| video.deselect());
    }

    pub fn current_timestamp(&self) -> Option<i64> {
        let inner = self.0.lock();
        inner
            .current_index
            .and_then(|current_index| inner.videos.get(current_index))
            .map(|video| video.timestamp())
    }

    pub fn update_last_played_timestamp(&self, last_played_timestamp: i64) {
        self.mutate_every_video(|video| video.select_based_on_timestamp(last_played_timestamp));
    }

    pub fn selected_videos(&self) -> Vec<VideoListItem> {
        let inner = self.0.lock();
        inner
            .videos
            .iter()
            .cloned()
            .filter_map(|video| video.selected.then_some(video))
            .collect()
    }

    pub fn list(&self, area: Rect) -> List<'_> {
        let inner = self.0.lock();
        let items = if let Some(current_index) = inner.current_index {
            generate_items(area, current_index, inner.videos.to_vec(), |video| {
                let selected = if video.selected { "✓" } else { " " };
                let width: usize = area.width.into();
                let label = video.label(width - 2);
                format!("{selected} {label}")
            })
        } else {
            Vec::new()
        };

        List::new(items)
            .block(Block::default().title("Videos"))
            .style(Style::default().fg(Color::White))
    }

    pub fn current_description(&self) -> Paragraph<'_> {
        let inner = self.0.lock();
        let description = inner
            .current_index
            .and_then(|current_index| inner.videos.get(current_index))
            .map(|video| video.description())
            .unwrap_or_default();

        Paragraph::new(description)
            .block(Block::default().title("Description").borders(Borders::TOP))
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true })
    }

    // Mutates the index and clamps it to the available video indexes
    fn mutate_current_index(&self, f: impl Fn(usize) -> usize) {
        let mut inner = self.0.lock();
        inner.current_index = inner
            .current_index
            .map(|current_index| f(current_index).clamp(0, inner.videos.len() - 1));
    }

    fn mutate_every_video(&self, f: impl Fn(&mut VideoListItem)) {
        let mut inner = self.0.lock();
        inner
            .videos
            .mutate_vec(|videos| videos.iter_mut().for_each(f));
    }

    fn mutate_current_video(&self, f: impl FnOnce(&mut VideoListItem)) {
        let mut inner = self.0.lock();
        let Some(current_index) = inner.current_index else { return };
        inner
            .videos
            .mutate_vec(|videos| videos.get_mut(current_index).map(f));
    }
}

pub struct FeedView {
    actions: Arc<ErrorHandlerActions>,
    common_config: Arc<CommonConfigHandler>,
    playing: Arc<Mutex<bool>>,
    video_list: Arc<VideoList>,
}

impl FeedView {
    pub fn new(
        actions: ErrorHandlerActions,
        common_config: CommonConfigHandler,
        config: Arc<impl Config + Send + Sync + 'static>,
    ) -> Self {
        let feed_view = Self {
            actions: Arc::new(actions),
            common_config: Arc::new(common_config),
            playing: Arc::new(Mutex::new(false)),
            video_list: Arc::new(VideoList::new()),
        };

        feed_view.listen_config_messages(config);
        feed_view
    }

    fn listen_config_messages(&self, config: Arc<impl Config + Send + Sync + 'static>) {
        let actions = self.actions.clone();
        let common_config = self.common_config.clone();
        let video_list = self.video_list.clone();
        tokio::spawn(async move {
            let mut receiver = config.subscribe();
            while let Ok(message) = receiver.recv().await {
                if let ConfigMessage::Error(error) = message {
                    actions.handle_error_async(error, true).await;
                } else {
                    video_list.handle_config_message(
                        message,
                        common_config.clone().last_played_timestamp(),
                    );
                    actions.redraw_async().await;
                }
            }
        });
    }

    fn set_current_as_last_played(&mut self) {
        let Some(last_played_timestamp) = self.video_list.current_timestamp() else { return };
        self.update_last_played_timestamp(last_played_timestamp);
    }

    fn update_last_played_timestamp(&mut self, last_played_timestamp: i64) {
        self.video_list
            .update_last_played_timestamp(last_played_timestamp);

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
        let selected_videos = self.video_list.selected_videos();

        if let Some(newest_video) = selected_videos.first() {
            self.update_last_played_timestamp(newest_video.timestamp());

            {
                let mut playing = self.playing.lock();
                *playing = true;
            }
            self.actions.redraw();

            let player = self.get_player();
            let playing = self.playing.clone();
            let actions = self.actions.clone();
            tokio::spawn(async move {
                let videos = selected_videos.iter().map(|video| video.url());
                let play_result = Command::new(player)
                    .args(videos)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .await;

                {
                    let mut playing = playing.lock();
                    *playing = false;
                }
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

    fn is_playing(&self) -> bool {
        *self.playing.lock()
    }
}

impl Component for FeedView {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        let description_height = 10;
        let description_y = area.height - description_height;
        let list_area = Rect::new(area.x, 0, area.width, description_y - 1);
        let description_area = Rect::new(area.x, description_y, area.width, description_height);

        let list = self.video_list.list(list_area);
        let description = self.video_list.current_description();

        f.render_widget(list, list_area);
        f.render_widget(description, description_area);

        if *self.playing.lock() {
            Dialog::new("Playing selection.").draw(f, area);
        }
    }

    fn handle_event(&mut self, event: Event) {
        if self.is_playing() {
            if event == Event::Key(KeyEvent::from(KeyCode::Esc)) {
                let mut playing = self.playing.lock();
                *playing = false;
                self.actions.redraw();
            }
        } else if let Event::Key(event) = event {
            match event.code {
                KeyCode::Up => self.video_list.move_up(),
                KeyCode::Down => self.video_list.move_down(),
                KeyCode::Char('j') => self.video_list.move_down(),
                KeyCode::Char('k') => self.video_list.move_up(),
                KeyCode::Char('g') => self.video_list.move_top(),
                KeyCode::Char('G') => self.video_list.move_bottom(),
                KeyCode::Char('a') => self.video_list.deselect_all(),
                KeyCode::Char(' ') => self.video_list.toggle_current(),
                KeyCode::Char('p') => self.play(),
                KeyCode::Char('n') => self.set_current_as_last_played(),
                _ => return,
            }
        }

        self.actions.redraw();
    }

    fn registered_events(&self) -> Vec<(String, String)> {
        if self.is_playing() {
            vec![(String::from("Esc"), String::from("Close"))]
        } else {
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
}
