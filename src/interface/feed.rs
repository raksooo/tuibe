use crate::feed::Feed as VideoFeed;
use crate::config::ConfigHandler;
use tui::layout::Rect;
use tui::widgets::{Block, Borders, List, ListItem};
use tui::style::{Color, Style};

pub struct Feed {
    feed: VideoFeed,
}

impl Feed {
    pub async fn new(config_handler: ConfigHandler) -> Feed {
        let feed = VideoFeed::from_config(config_handler.config)
            .await
            .expect("Failed to fetch videos");
        Feed {
            feed,
        }
    }

    pub fn render(&self, size: Rect) -> List<'static> {
        let items: Vec<ListItem> = self.feed.videos
            .iter()
            .map(|video| ListItem::new(video.get_label(size.width.into())))
            .collect();

        List::new(items)
            .block(Block::default().title("List").borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Green))
    }
}
