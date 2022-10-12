use crate::config::ConfigHandler;
use crate::interface::feed::Feed;
use crossterm::event::{Event, KeyCode};

pub struct App {
    config_handler: ConfigHandler,
    pub feed: Feed,
}

impl App {
    pub async fn new(config_handler: ConfigHandler) -> App {
        let feed = Feed::new(&config_handler.config).await;
        App {
            config_handler,
            feed,
        }
    }

    pub fn handle_event(&mut self, event: Event) -> bool {
        if let Event::Key(event) = event {
            match event.code {
                KeyCode::Char('q') => return false,
                KeyCode::Char(' ') => self.feed.toggle_current_item(),
                KeyCode::Up => self.feed.move_up(),
                KeyCode::Char('k') => self.feed.move_up(),
                KeyCode::Down => self.feed.move_down(),
                KeyCode::Char('j') => self.feed.move_down(),
                _ => (),
            }
        }

        true
    }
}
