use crate::config::ConfigHandler;
use crate::interface::component::{Component, EventFuture, Frame, UpdateEvent, UpdateSender};
use crate::interface::feed::Feed;
use crossterm::event::{Event, KeyCode, KeyEvent};
use std::sync::Arc;
use tokio::sync::Mutex;
use tui::layout::Rect;

pub struct App {
    config_handler: Arc<Mutex<Option<ConfigHandler>>>,

    tx: UpdateSender,
    pub feed: Arc<Mutex<Feed>>,
}

impl App {
    pub fn new(tx: UpdateSender) -> Self {
        let new_app = Self::create_empty(tx.clone());

        let config_handler = Arc::clone(&new_app.config_handler);
        let feed = Arc::clone(&new_app.feed);
        tokio::spawn(async move {
            {
                let mut config_handler = config_handler.lock().await;
                *config_handler = None;
            }

            let new_config_handler = ConfigHandler::load().await.expect("Failed to load config");
            let new_config = new_config_handler.config.clone();

            {
                let mut config_handler = config_handler.lock().await;
                *config_handler = Some(new_config_handler);
            }

            {
                let mut feed = feed.lock().await;
                feed.update_with_config(&new_config).await;
            }

            tx.send(UpdateEvent::Redraw).await;
        });

        new_app
    }

    fn create_empty(tx: UpdateSender) -> Self {
        let config_handler = Arc::new(Mutex::new(None));
        let feed = Arc::new(Mutex::new(Feed::new(tx.clone(), None)));

        Self {
            config_handler,
            tx,
            feed,
        }
    }
}

impl Component for App {
    fn draw(&mut self, f: &mut Frame, size: Rect) {
        if let Ok(mut feed) = self.feed.try_lock() {
            feed.draw(f, size);
        }
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
            let feed = Arc::clone(&self.feed);
            Box::pin(async move {
                let mut feed = feed.lock().await;
                feed.handle_event(event).await;
            })
        }
    }
}
