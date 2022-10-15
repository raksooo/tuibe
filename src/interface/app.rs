use crate::config::ConfigHandler;
use crate::interface::component::{Component, EventFuture, Frame};
use crate::interface::feed::Feed;
use crossterm::event::Event;
use tui::layout::Rect;

pub struct App {
    config_handler: ConfigHandler,

    pub feed: Box<dyn Component>,
}

impl App {
    pub fn new(config_handler: ConfigHandler) -> Self {
        let feed = Feed::new(&config_handler.config);
        Self {
            config_handler,
            feed: Box::new(feed),
        }
    }
}

impl Component for App {
    fn draw(&mut self, f: &mut Frame, size: Rect) {
        self.feed.draw(f, size);
    }

    fn handle_event(&mut self, event: Event) -> EventFuture {
        self.feed.handle_event(event)
    }
}
