use crate::config::ConfigHandler;
use crate::interface::component::{Component, Frame};
use crate::interface::feed::Feed;
use crossterm::event::Event;
use tui::layout::Rect;

pub struct App {
    config_handler: ConfigHandler,
    pub feed: Box<dyn Component>,
}

impl App {
    pub async fn new(config_handler: ConfigHandler) -> App {
        let feed = Feed::new(&config_handler.config).await;
        App {
            config_handler,
            feed: Box::new(feed),
        }
    }
}

impl Component for App {
    fn draw(&self, f: &mut Frame, size: Rect) {
        self.feed.draw(f, size);
    }

    // TODO: Spawn process for reloading feed. Shouldn't block.
    fn handle_event(&mut self, event: Event) {
        self.feed.handle_event(event);
    }
}
