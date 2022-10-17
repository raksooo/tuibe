use crate::{
    config::ConfigHandler,
    interface::{
        component::{Component, EventFuture, EventSender, Frame, UpdateEvent},
        dialog,
        feed::Feed,
        loading_indicator::LoadingIndicator,
    },
};
use crossterm::event::{Event, KeyCode};
use std::sync::Arc;
use tokio::sync::Mutex;
use tui::layout::Rect;

pub struct App {
    show_subscriptions: Arc<Mutex<bool>>,
    config_handler: Arc<Mutex<Result<Option<ConfigHandler>, ()>>>,

    tx: EventSender,
    loading_indicator: LoadingIndicator,
    feed: Arc<Mutex<Feed>>,
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
        let loading_indicator = LoadingIndicator::new(tx.clone());

        Self {
            show_subscriptions: Arc::new(Mutex::new(false)),
            config_handler,

            tx,
            loading_indicator,
            feed,
        }
    }

    fn quit(&self) -> EventFuture {
        let tx = self.tx.clone();
        Box::pin(async move {
            let _ = tx.send(UpdateEvent::Quit).await;
        })
    }

    fn toggle_show_subscriptions(&self) -> EventFuture {
        let show_subscriptions = Arc::clone(&self.show_subscriptions);
        Box::pin(async move {
            let mut show_subscriptions = show_subscriptions.lock().await;
            *show_subscriptions = !*show_subscriptions;
        })
    }
}

impl Component for App {
    fn draw(&mut self, f: &mut Frame, size: Rect) {
        if let Ok(config_handler) = self.config_handler.try_lock() {
            if let Ok(_) = *config_handler {
                if let Ok(mut feed) = self.feed.try_lock() {
                    feed.draw(f, size);
                } else {
                    self.loading_indicator.draw(f, size);
                }
            } else {
                dialog::dialog(f, size, "Something went wrong..");
            }
        } else {
            dialog::dialog(f, size, "Something went wrong..");
        }
    }

    fn handle_event(&mut self, event: Event) -> EventFuture {
        if let Event::Key(event) = event {
            match event.code {
                KeyCode::Char('q') => return self.quit(),
                KeyCode::Char('s') => return self.toggle_show_subscriptions(),
                _ => (),
            }
        }

        let feed = Arc::clone(&self.feed);
        Box::pin(async move {
            let mut feed = feed.lock().await;
            feed.handle_event(event).await;
        })
    }
}
