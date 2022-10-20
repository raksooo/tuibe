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
use tui::layout::{Constraint, Direction, Layout, Rect};

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

    fn toggle_subscriptions(&self) -> UpdateEvent {
        let config_handler = self.config_handler.lock().unwrap();
        let mut subscriptions = self.subscriptions.lock().unwrap();
        if subscriptions.is_some() {
            *subscriptions = None;
        } else {
            if let Some(ref config_handler) = *config_handler {
                if let Some(config_data) = &config_handler.config_data {
                    *subscriptions = Some(Subscriptions::new(
                        self.tx.clone(),
                        config_data.channels.clone(),
                    ));
                }
            }
        }

        UpdateEvent::Redraw
    }
}

impl Component for App {
    fn draw(&mut self, f: &mut Frame, size: Rect) {
        let mut subscriptions = self.subscriptions.lock().unwrap();
        let subscriptions_numerator = subscriptions.as_ref().map_or(0, |_| 1);

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Ratio(subscriptions_numerator, 2),
                    Constraint::Ratio(2 - subscriptions_numerator, 2),
                ]
                .as_ref(),
            )
            .split(size);

        if let Some(ref mut subscriptions) = *subscriptions {
            subscriptions.draw(f, chunks[0]);
        }

        let mut feed = self.feed.lock().unwrap();
        feed.draw(f, chunks[1]);
    }

    fn handle_event(&mut self, event: Event) -> UpdateEvent {
        if let Event::Key(event) = event {
            match event.code {
                KeyCode::Char('q') => return UpdateEvent::Quit,
                KeyCode::Char('s') => return self.toggle_subscriptions(),
                _ => (),
            }
        }

        let mut subscriptions = self.subscriptions.lock().unwrap();
        if let Some(ref mut subscriptions) = *subscriptions {
            subscriptions.handle_event(event)
        } else {
            let mut feed = self.feed.lock().unwrap();
            feed.handle_event(event)
        }
    }
}
