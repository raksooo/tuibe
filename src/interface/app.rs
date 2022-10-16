use crate::{
    config::ConfigHandler,
    interface::{
        component::{handled_event, Component, EventFuture, EventSender, Frame, UpdateEvent},
        dialog,
        feed::Feed,
        loading_indicator::LoadingIndicator,
    },
};
use crossterm::event::{Event, KeyCode, KeyEvent};
use std::sync::Arc;
use tokio::sync::Mutex;
use tui::layout::Rect;

pub struct App {
    config_handler: Arc<Mutex<Result<Option<ConfigHandler>, ()>>>,

    pub loading_indicator: LoadingIndicator,
    pub feed: Arc<Mutex<Feed>>,
}

impl App {
    pub fn new(tx: EventSender) -> Self {
        let new_app = Self::create_empty(tx.clone());

        let config_handler = Arc::clone(&new_app.config_handler);
        let feed = Arc::clone(&new_app.feed);
        tokio::spawn(async move {
            {
                let mut config_handler = config_handler.lock().await;
                *config_handler = Ok(None);
            }

            if let Ok(new_config_handler) = ConfigHandler::load().await {
                let new_config = new_config_handler.config.clone();
                {
                    let mut config_handler = config_handler.lock().await;
                    *config_handler = Ok(Some(new_config_handler));
                }
                {
                    let mut feed = feed.lock().await;
                    feed.update_with_config(&new_config).await;
                }
            } else {
                let mut config_handler = config_handler.lock().await;
                *config_handler = Err(());
            }

            let _ = tx.send(UpdateEvent::Redraw).await;
        });

        new_app
    }

    fn create_empty(tx: EventSender) -> Self {
        let config_handler = Arc::new(Mutex::new(Ok(None)));
        let feed = Arc::new(Mutex::new(Feed::new(tx.clone(), None)));
        let loading_indicator = LoadingIndicator::new(tx);

        Self {
            config_handler,
            loading_indicator,
            feed,
        }
    }
}

impl Component for App {
    fn draw(&mut self, f: &mut Frame, size: Rect) {
        if let Ok(config_handler) = self.config_handler.try_lock() {
            if let Ok(_) = *config_handler {
                if let Ok(mut feed) = self.feed.try_lock() {
                    feed.draw(f, size);
                    return;
                } else {
                    self.loading_indicator.draw(f, size);
                    return;
                }
            }
        }

        dialog::dialog(f, size, "Something went wrong..");
    }

    fn handle_event(&mut self, event: Event) -> EventFuture {
        if let Event::Key(KeyEvent {
            code: KeyCode::Char('q'),
            ..
        }) = event
        {
            handled_event(UpdateEvent::Quit)
        } else {
            let feed = Arc::clone(&self.feed);
            Box::pin(async move {
                let mut feed = feed.lock().await;
                feed.handle_event(event).await
            })
        }
    }
}
