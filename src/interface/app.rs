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
    config_handler: Arc<Mutex<Option<ConfigHandler>>>,

    tx: EventSender,
    feed: Arc<Mutex<Box<dyn Component + Send>>>,
    subscriptions: Arc<Mutex<Option<Subscriptions>>>,
}

impl App {
    pub fn new(tx: EventSender) -> Self {
        let mut app = Self {
            config_handler: Arc::new(Mutex::new(None)),

            tx: tx.clone(),
            feed: Arc::new(Mutex::new(Box::new(LoadingIndicator::new(tx.clone())))),
            subscriptions: Arc::new(Mutex::new(None)),
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

    fn toggle_subscriptions(&self) {
        let config_handler = Arc::clone(&self.config_handler);
        let subscriptions = Arc::clone(&self.subscriptions);
        let tx = self.tx.clone();

        tokio::spawn(async move {
            {
                let config_handler = config_handler.lock().unwrap();
                let mut subscriptions = subscriptions.lock().unwrap();
                if subscriptions.is_some() {
                    *subscriptions = None;
                } else {
                    if let Some(ref config_handler) = *config_handler {
                        if let Some(config_data) = &config_handler.config_data {
                            *subscriptions =
                                Some(Subscriptions::new(tx.clone(), config_data.channels.clone()));
                        }
                    }
                }
            }
            let _ = tx.send(UpdateEvent::Redraw).await;
        });
    }
}

impl Component for App {
    fn draw(&mut self, f: &mut Frame, size: Rect) {
        let mut subscriptions = self.subscriptions.lock().unwrap();
        let subscriptions_width = if subscriptions.is_some() { size.width / 2 } else { 0 };
        {
            let feed_size = Rect::new(subscriptions_width, 0, size.width - subscriptions_width, size.height);
            let mut feed = self.feed.lock().unwrap();
            feed.draw(f, feed_size);
        }

        if let Some(ref mut subscriptions) = *subscriptions {
            let size = Rect::new(0, 0, subscriptions_width, size.height);
            subscriptions.draw(f, size);
        }
    }

    fn handle_event(&mut self, event: Event) {
        if let Event::Key(event) = event {
            match event.code {
                KeyCode::Char('q') => return self.quit(),
                KeyCode::Char('s') => return self.toggle_subscriptions(),
                _ => (),
            }
        }

        let mut subscriptions = self.subscriptions.lock().unwrap();
        if let Some(ref mut subscriptions) = *subscriptions {
            subscriptions.handle_event(event);
        } else {
            let mut feed = self.feed.lock().unwrap();
            feed.handle_event(event);
        }
    }
}
