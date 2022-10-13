use crate::config::Config;
use crate::feed::{Feed as VideoFeed, Video};
use crate::interface::component::{Component, Frame};
use crossterm::event::{Event, KeyCode};
use tui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

pub struct Feed {
    feed: Vec<Video>,
    current_item: usize,
}

impl Feed {
    pub async fn new(config: &Config) -> Feed {
        Feed {
            feed: Feed::load_feed_from_config(config).await,
            current_item: 0,
        }
    }

    pub async fn reload_feed(&mut self, config: &Config) {
        self.feed = Feed::load_feed_from_config(config).await;
    }

    pub fn toggle_current_item(&mut self) {
        if let Some(video) = self.feed.get_mut(self.current_item) {
            video.toggle_selected();
        }
    }

    pub fn move_up(&mut self) {
        if self.current_item > 0 {
            self.current_item -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.current_item < self.feed.len() - 1 {
            self.current_item += 1;
        }
    }

    fn create_list(&self, width: usize) -> List {
        let mut items: Vec<ListItem> = Vec::new();

        for (i, video) in self.feed.iter().enumerate() {
            let mut item = ListItem::new(video.get_label(width));
            if i == self.current_item {
                item = item.style(Style::default().fg(Color::Green));
            }
            items.push(item);
        }

        List::new(items)
            .block(Block::default().title("Videos"))
            .style(Style::default().fg(Color::White))
    }

    fn create_description(&self) -> Paragraph {
        let description = self
            .feed
            .get(self.current_item)
            .unwrap()
            .description
            .to_owned();

        Paragraph::new(description)
            .block(Block::default().title("Description").borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true })
    }

    async fn load_feed_from_config(config: &Config) -> Vec<Video> {
        VideoFeed::from_config(config)
            .await
            .expect("Failed to fetch videos")
            .videos
    }
}

impl Component for Feed {
    fn draw(&self, f: &mut Frame, size: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(80), Constraint::Percentage(20)].as_ref())
            .split(size);

        let width = f.size().width.into();
        let list = self.create_list(width);
        let description = self.create_description();

        f.render_widget(list, chunks[0]);
        f.render_widget(description, chunks[1]);
    }

    fn handle_event(&mut self, event: Event) {
        if let Event::Key(event) = event {
            match event.code {
                KeyCode::Char(' ') => self.toggle_current_item(),
                KeyCode::Up => self.move_up(),
                KeyCode::Down => self.move_down(),
                KeyCode::Char('j') => self.move_down(),
                KeyCode::Char('k') => self.move_up(),
                _ => (),
                // KeyCode::Char('r') => self.reload_feed(&self.config_handler.config).await,
            }
        }
    }
}
