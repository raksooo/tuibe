use crate::{
    config::ConfigHandler,
    interface::{
        component::{Component, EventSender, Frame, UpdateEvent},
        dialog,
        feed::Feed,
    },
};
use crossterm::event::{Event, KeyCode};
use std::sync::{Arc, Mutex};
use tui::layout::Rect;

pub struct App {
    show_subscriptions: Arc<Mutex<bool>>,
    config_handler: Arc<Mutex<Result<Option<ConfigHandler>, ()>>>,

    tx: EventSender,
    feed: Arc<Mutex<Feed>>,
}

impl App {
    pub fn new(tx: EventSender) -> Self {
        let config_handler = Arc::new(Mutex::new(Ok(None)));
        let feed = Arc::new(Mutex::new(Feed::new(tx.clone(), None)));

        let mut new_app = Self {
            show_subscriptions: Arc::new(Mutex::new(false)),
            config_handler,
            tx,
            feed,
        };

        new_app.reload();
        new_app
    }

    fn reload(&mut self) {
        let tx = self.tx.clone();
        let config_handler = Arc::clone(&self.config_handler);
        let feed = Arc::clone(&self.feed);

        tokio::spawn(async move {
            {
                let mut config_handler = config_handler.lock().unwrap();
                *config_handler = Ok(None);
            }

            if let Ok(new_config_handler) = ConfigHandler::load().await {
                let new_config = new_config_handler.config.clone();
                {
                    let mut config_handler = config_handler.lock().unwrap();
                    *config_handler = Ok(Some(new_config_handler));
                }
                {
                    let mut feed = feed.lock().unwrap();
                    feed.update_with_config(&new_config);
                }
            } else {
                let mut config_handler = config_handler.lock().unwrap();
                *config_handler = Err(());
            }

            let _ = tx.send(UpdateEvent::Redraw).await;
        });
    }

    fn quit(&self) {
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let _ = tx.send(UpdateEvent::Quit).await;
        });
    }

    fn toggle_show_subscriptions(&self) {
        let show_subscriptions = Arc::clone(&self.show_subscriptions);
        let tx = self.tx.clone();

        tokio::spawn(async move {
            {
                let mut show_subscriptions = show_subscriptions.lock().unwrap();
                *show_subscriptions = !*show_subscriptions;
            }
            let _ = tx.send(UpdateEvent::Redraw).await;
        });
    }
}

impl Component for App {
    fn draw(&mut self, f: &mut Frame, size: Rect) {
        if let Ok(_) = *self.config_handler.lock().unwrap() {
            let mut feed = self.feed.lock().unwrap();
            feed.draw(f, size);
        } else {
            dialog::dialog(f, size, "Something went wrong..");
        }
    }

    fn handle_event(&mut self, event: Event) {
        if let Event::Key(event) = event {
            match event.code {
                KeyCode::Char('q') => return self.quit(),
                KeyCode::Char('s') => return self.toggle_show_subscriptions(),
                _ => (),
            }
        }

        let mut feed = self.feed.lock().unwrap();
        feed.handle_event(event);
    }
}
