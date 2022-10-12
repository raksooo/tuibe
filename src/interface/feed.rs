use crate::config::Config;
use crate::feed::Feed as VideoFeed;
use tui::{
    backend::Backend,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub struct Feed {
    feed: VideoFeed,
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
        if let Some(video) = self.feed.videos.get_mut(self.current_item) {
            video.toggle_selected();
        }
    }

    pub fn move_up(&mut self) {
        if self.current_item > 0 {
            self.current_item -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.current_item < self.feed.videos.len() - 1 {
            self.current_item += 1;
        }
    }

    pub fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        list_constraints: Rect,
        description_constrints: Rect,
    ) {
        let width = f.size().width.into();
        let list = self.create_list(width);
        let description = self.create_description();

        f.render_widget(list, list_constraints);
        f.render_widget(description, description_constrints);
    }

    fn create_list(&self, width: usize) -> List {
        let mut items: Vec<ListItem> = Vec::new();

        for (i, video) in self.feed.videos.iter().enumerate() {
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
            .videos
            .get(self.current_item)
            .unwrap()
            .description
            .to_owned();

        Paragraph::new(description)
            .block(Block::default().title("Description").borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true })
    }

    async fn load_feed_from_config(config: &Config) -> VideoFeed {
        VideoFeed::from_config(config)
            .await
            .expect("Failed to fetch videos")
    }
}
