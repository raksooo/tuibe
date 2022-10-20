use crate::{
    config::ConfigHandler,
    interface::{
        component::{Component, EventSender, Frame, UpdateEvent},
        dialog::Dialog,
        feed::Feed,
        loading_indicator::LoadingIndicator,
        subscriptions::Subscriptions,
    },
};
use crossterm::event::{Event, KeyCode};
use std::sync::{Arc, Mutex};
use tui::layout::Rect;

pub struct App {
    show_subscriptions: Arc<Mutex<bool>>,
    config_handler: Arc<Mutex<Option<ConfigHandler>>>,

    tx: EventSender,
    feed: Arc<Mutex<Box<dyn Component + Send>>>,
    subscriptions: Arc<Mutex<Subscriptions>>,
}

impl App {
    pub fn new(tx: EventSender) -> Self {
        let mut app = Self {
            show_subscriptions: Arc::new(Mutex::new(false)),
            config_handler: Arc::new(Mutex::new(None)),

            tx: tx.clone(),
            feed: Arc::new(Mutex::new(Box::new(LoadingIndicator::new(tx.clone())))),
            subscriptions: Arc::new(Mutex::new(Subscriptions::new(tx.clone()))),
        };

        app.init();
        app
    }

    fn init(&mut self) {
        let tx = self.tx.clone();
        let config_handler = Arc::clone(&self.config_handler);
        let feed = Arc::clone(&self.feed);

        tokio::spawn(async move {
            if let Ok(mut new_config_handler) = ConfigHandler::load().await {
                if let Ok(()) = new_config_handler.fetch().await {
                    if let Some(config_data) = &new_config_handler.config_data {
                        let mut feed = feed.lock().unwrap();
                        *feed = Box::new(Feed::new(
                            tx.clone(),
                            config_data.videos.clone().into_iter().collect(),
                        ));
                    } else {
                        let mut feed = feed.lock().unwrap();
                        *feed = Box::new(Dialog::new("Something went wrong.."));
                    }
                } else {
                    let mut feed = feed.lock().unwrap();
                    *feed = Box::new(Dialog::new("Something went wrong.."));
                }

                {
                    let mut config_handler = config_handler.lock().unwrap();
                    *config_handler = Some(new_config_handler);
                }
            } else {
                let mut feed = feed.lock().unwrap();
                *feed = Box::new(Dialog::new("Something went wrong.."));
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
        let mut feed = self.feed.lock().unwrap();
        feed.draw(f, size);
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
