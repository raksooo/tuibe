use super::{
    component::{Component, Frame},
    dialog::Dialog,
    main_view::MainViewActions,
};
use crate::config::{common::CommonConfigHandler, Video};

use crossterm::event::{Event, KeyCode, KeyEvent};
use parking_lot::Mutex;
use std::{env, process::Stdio, sync::Arc};
use tokio::process::Command;
use tui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

struct VideoListItem {
    pub video: Video,
    pub selected: bool,
}

pub struct FeedView {
    actions: Arc<MainViewActions>,
    common_config: Arc<CommonConfigHandler>,
    playing: Arc<Mutex<bool>>,
    videos: Vec<VideoListItem>,
    current_item: usize,
}

impl FeedView {
    pub fn new(
        actions: MainViewActions,
        common_config: CommonConfigHandler,
        videos: Vec<Video>,
    ) -> Self {
        let last_played_timestamp = common_config.config().last_played_timestamp;
        let videos = videos
            .into_iter()
            .map(|video| {
                let timestamp = video.date.timestamp();
                VideoListItem {
                    video,
                    selected: timestamp > last_played_timestamp,
                }
            })
            .collect();

        Self {
            actions: Arc::new(actions),
            common_config: Arc::new(common_config),
            playing: Arc::new(Mutex::new(false)),
            videos,
            current_item: 0,
        }
    }

    fn move_up(&mut self) {
        if self.current_item > 0 {
            self.current_item -= 1;
            self.actions.redraw();
        }
    }

    fn move_top(&mut self) {
        self.current_item = 0;
        self.actions.redraw();
    }

    fn move_down(&mut self) {
        if self.current_item + 1 < self.videos.len() {
            self.current_item += 1;
            self.actions.redraw();
        }
    }

    fn toggle_current_item(&mut self) {
        if let Some(video) = self.videos.get_mut(self.current_item) {
            video.selected = !video.selected;
            self.actions.redraw();
        }
    }

    fn update_last_played_timestamp(&mut self, last_played_timestamp: i64) {
        for mut video in self.videos.iter_mut() {
            video.selected = video.video.date.timestamp() > last_played_timestamp;
        }

        let common_config = Arc::clone(&self.common_config);
        let actions = Arc::clone(&self.actions);
        tokio::spawn(async move {
            actions.handle_result(
                common_config
                    .set_last_played_timestamp(last_played_timestamp)
                    .await,
            );
        });
    }

    fn play(&mut self) {
        let selected_videos: Vec<Video> = self
            .videos
            .iter()
            .filter(|video| video.selected)
            .map(|video| video.video.clone())
            .collect();

        if let Some(newest_video) = selected_videos.first() {
            self.update_last_played_timestamp(newest_video.date.timestamp());

            {
                let mut playing = self.playing.lock();
                *playing = true;
            }
            self.actions.redraw();

            let player = self.get_player();
            let playing = Arc::clone(&self.playing);
            let actions = Arc::clone(&self.actions);
            tokio::spawn(async move {
                let videos = selected_videos.iter().map(|video| &video.url);
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
        match env::args().skip_while(|arg| arg != "--player").nth(1) {
            Some(player) => player,
            None => self.common_config.config().player,
        }
    }

    fn is_playing(&self) -> bool {
        *self.playing.lock()
    }

    fn create_list(&self, area: Rect) -> List<'_> {
        let mut items: Vec<ListItem> = Vec::new();

        let height: usize = area.height.into();
        let nvideos = self.videos.len();
        let start_index = if self.current_item < height / 2 {
            0
        } else if self.current_item >= nvideos - height / 2 {
            nvideos - height + 1
        } else {
            self.current_item - (height / 2)
        };

        for (i, video) in self.videos.iter().skip(start_index).enumerate() {
            let selected = if video.selected { "✓" } else { " " };
            let width: usize = area.width.into();
            let label = video.video.get_label(width - 2);
            let mut item = ListItem::new(format!("{selected} {label}"));

            if i + start_index == self.current_item {
                item = item.style(Style::default().fg(Color::Green));
            }

            items.push(item);
        }

        List::new(items)
            .block(Block::default().title("Videos"))
            .style(Style::default().fg(Color::White))
    }

    fn create_description(&self) -> Paragraph<'_> {
        // current_item is always within the bounds of videos
        let description: &str = self
            .videos
            .get(self.current_item)
            .map(|video| video.video.description.as_str())
            .unwrap_or_default();

        Paragraph::new(description)
            .block(Block::default().title("Description").borders(Borders::TOP))
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true })
    }
}

impl Component for FeedView {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        let description_height = 10;
        let description_y = area.height - description_height;
        let list_area = Rect::new(area.x, 0, area.width, description_y - 1);
        let description_area = Rect::new(area.x, description_y, area.width, description_height);

        let list = self.create_list(list_area);
        let description = self.create_description();

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
                KeyCode::Up => self.move_up(),
                KeyCode::Down => self.move_down(),
                KeyCode::Char('j') => self.move_down(),
                KeyCode::Char('k') => self.move_up(),
                KeyCode::Char('g') => self.move_top(),
                KeyCode::Char(' ') => self.toggle_current_item(),
                KeyCode::Char('p') => self.play(),
                _ => (),
            }
        }
    }

    fn registered_events(&self) -> Vec<(String, String)> {
        if self.is_playing() {
            vec![(String::from("Esc"), String::from("Close"))]
        } else {
            vec![
                (String::from("j"), String::from("Down")),
                (String::from("k"), String::from("Up")),
                (String::from("g"), String::from("Top")),
                (String::from("Space"), String::from("Select")),
                (String::from("p"), String::from("Play")),
            ]
        }
    }
}
