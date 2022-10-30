use super::{
    app::AppMsg,
    component::{Component, Frame, UpdateEvent},
};
use crate::{config::config::Video, sender_ext::SenderExt};
use crossterm::event::{Event, KeyCode};
use tokio::sync::mpsc;
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
    app_sender: mpsc::Sender<AppMsg>,

    videos: Vec<VideoListItem>,
    current_item: usize,
}

impl FeedView {
    pub fn new(
        app_sender: mpsc::Sender<AppMsg>,
        videos: Vec<Video>,
        last_played_timestamp: i64,
    ) -> Self {
        let videos = videos
            .iter()
            .map(|video| VideoListItem {
                video: video.to_owned(),
                selected: video.date.timestamp() > last_played_timestamp,
            })
            .collect();

        Self {
            app_sender,
            videos,
            current_item: 0,
        }
    }

    pub fn update_last_played_timestamp(&mut self, last_played_timestamp: i64) {
        for mut video in self.videos.iter_mut() {
            video.selected = video.video.date.timestamp() > last_played_timestamp;
        }
    }

    fn move_up(&mut self) -> UpdateEvent {
        if self.current_item > 0 {
            self.current_item -= 1;
        }
        UpdateEvent::Redraw
    }

    fn move_down(&mut self) -> UpdateEvent {
        if self.current_item + 1 < self.videos.len() {
            self.current_item += 1;
        }
        UpdateEvent::Redraw
    }

    fn toggle_current_item(&mut self) -> UpdateEvent {
        if let Some(video) = self.videos.get_mut(self.current_item) {
            video.selected = !video.selected;
        }
        UpdateEvent::Redraw
    }

    fn play(&self) -> UpdateEvent {
        let selected_videos = self
            .videos
            .iter()
            .filter(|video| video.selected)
            .map(|video| video.video.clone());
        self.app_sender
            .send_sync(AppMsg::Play(selected_videos.collect()));
        UpdateEvent::None
    }

    fn create_list(&self, width: usize) -> List<'_> {
        let mut items: Vec<ListItem> = Vec::new();

        for (i, video) in self.videos.iter().enumerate() {
            let selected = if video.selected { "✓" } else { " " };
            let label = video.video.get_label(width);
            let mut item = ListItem::new(format!("{selected} {label}"));

            if i == self.current_item {
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
        let description = if let Some(video) = self.videos.get(self.current_item) {
            video.video.description.to_owned()
        } else {
            "".to_string()
        };

        Paragraph::new(description)
            .block(Block::default().title("Description").borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true })
    }
}

impl Component for FeedView {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        let description_height = 10;
        let description_y = area.height - description_height;
        let list_area = Rect::new(area.x, 0, area.width, description_y - 10);
        let description_area = Rect::new(area.x, description_y, area.width, description_height);

        let list = self.create_list(list_area.width.into());
        let description = self.create_description();

        f.render_widget(list, list_area);
        f.render_widget(description, description_area);
    }

    fn handle_event(&mut self, event: Event) -> UpdateEvent {
        if let Event::Key(event) = event {
            match event.code {
                KeyCode::Up => self.move_up(),
                KeyCode::Down => self.move_down(),
                KeyCode::Char('j') => self.move_down(),
                KeyCode::Char('k') => self.move_up(),
                KeyCode::Char(' ') => self.toggle_current_item(),
                KeyCode::Char('p') => self.play(),
                _ => UpdateEvent::None,
            }
        } else {
            UpdateEvent::None
        }
    }
}
