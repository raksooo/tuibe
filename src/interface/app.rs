use crate::config::{Config, ConfigHandler};
use crate::interface::component::{Component, EventFuture, Frame, UpdateSender};
use crate::interface::feed::Feed;
use crossterm::event::Event;
use tui::layout::Rect;

pub struct App {
    config_handler: ConfigHandler,

    tx: UpdateSender,
    pub feed: Box<dyn Component<Config>>,
}

impl Component<ConfigHandler> for App {
    fn new(tx: UpdateSender, config_handler: ConfigHandler) -> Self {
        let feed = Feed::new(tx.clone(), config_handler.config.to_owned());
        Self {
            config_handler,

            tx: tx,
            feed: Box::new(feed),
        }
    }

    fn draw(&mut self, f: &mut Frame, size: Rect) {
        self.feed.draw(f, size);
    }

    fn handle_event(&mut self, event: Event) -> EventFuture {
        self.feed.handle_event(event)
    }
}
