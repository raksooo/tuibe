use crate::config::{Config, ConfigHandler};
use crate::interface::component::{Component, EventFuture, Frame, UpdateEvent, UpdateSender};
use crate::interface::feed::Feed;
use crossterm::event::{Event, KeyCode, KeyEvent};
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

            tx,
            feed: Box::new(feed),
        }
    }

    fn draw(&mut self, f: &mut Frame, size: Rect) {
        self.feed.draw(f, size);
    }

    fn handle_event(&mut self, event: Event) -> EventFuture {
        if let Event::Key(KeyEvent {
            code: KeyCode::Char('q'),
            ..
        }) = event
        {
            let tx = self.tx.clone();
            Box::pin(async move {
                tx.send(UpdateEvent::Quit)
                    .await
                    .expect("Failed to send quit event");
            })
        } else {
            self.feed.handle_event(event)
        }
    }
}
