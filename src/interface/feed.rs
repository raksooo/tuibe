use crate::feed::{Feed as VideoFeed};
use crate::config::Config;
use tui::layout::Rect;
use tui::widgets::{Block, List, ListItem};
use tui::style::{Color, Style};

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

    pub fn render(&self, size: Rect) -> List {
        let mut items: Vec<ListItem> = Vec::new();

        for (i, video) in self.feed.videos.iter().enumerate() {
            let mut item = ListItem::new(video.get_label(size.width.into()));
            if i == self.current_item {
                item = item.style(Style::default().fg(Color::Green));
            }
            items.push(item);
        }

        List::new(items)
            .block(Block::default().title("Videos"))
            .style(Style::default().fg(Color::White))
    }

    async fn load_feed_from_config(config: &Config) -> VideoFeed {
        VideoFeed::from_config(config).await.expect("Failed to fetch videos")
    }
}
