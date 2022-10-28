use super::{
    component::{Component, Frame, UpdateEvent},
    config_provider::ConfigProviderMsg,
    feed_view::FeedView,
};
use crate::{
    config::{common::CommonConfigHandler, config::Video},
    sender_ext::SenderExt,
};
use crossterm::event::{Event, KeyCode};
use parking_lot::Mutex;
use std::sync::Arc;
use tokio::sync::mpsc;
use tui::layout::{Constraint, Direction, Layout, Rect};

pub enum AppMsg {
    CloseConfig,
}

pub struct App {
    show_config: Arc<Mutex<bool>>,

    feed: FeedView,
    config: Box<dyn Component + Send>,

    config_sender: mpsc::Sender<ConfigProviderMsg>,
}

impl App {
    pub fn new<C, CF>(
        config_sender: mpsc::Sender<ConfigProviderMsg>,
        common_config: CommonConfigHandler,
        videos: Vec<Video>,
        config_creator: CF,
    ) -> Self
    where
        C: Component + Send + 'static,
        CF: FnOnce(mpsc::Sender<AppMsg>) -> C,
    {
        let last_played_timestamp = common_config.config().last_played_timestamp;
        let (app_sender, app_receiver) = mpsc::channel(100);

        let new_app = Self {
            show_config: Arc::new(Mutex::new(false)),

            feed: FeedView::new(videos, last_played_timestamp),
            config: Box::new(config_creator(app_sender)),

            config_sender,
        };

        new_app.listen_app_msg(app_receiver);

        new_app
    }

    fn listen_app_msg(&self, mut app_receiver: mpsc::Receiver<AppMsg>) {
        let show_config = Arc::clone(&self.show_config);
        let config_sender = self.config_sender.clone();
        tokio::spawn(async move {
            loop {
                if let Some(msg) = app_receiver.recv().await {
                    match msg {
                        AppMsg::CloseConfig => {
                            let mut show_config = show_config.lock();
                            *show_config = false;
                            config_sender.send_sync(ConfigProviderMsg::Reload);
                        }
                    }
                } else {
                    break;
                }
            }
        });
    }

    fn set_show_config(&mut self) -> UpdateEvent {
        let mut show_config = self.show_config.lock();
        *show_config = true;
        UpdateEvent::Redraw
    }
}

impl Component for App {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        let show_config = { self.show_config.lock().to_owned() };
        let config_numerator = if show_config { 1 } else { 0 };

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Ratio(config_numerator, 2),
                    Constraint::Ratio(2 - config_numerator, 2),
                ]
                .as_ref(),
            )
            .split(area);

        if show_config {
            self.config.draw(f, chunks[0]);
        }

        self.feed.draw(f, chunks[1]);
    }

    fn handle_event(&mut self, event: Event) -> UpdateEvent {
        if let Event::Key(event) = event {
            match event.code {
                KeyCode::Char('q') => return UpdateEvent::Quit,
                KeyCode::Char('s') => return self.set_show_config(),
                _ => (),
            }
        }

        if *self.show_config.lock() {
            self.config.handle_event(event)
        } else {
            self.feed.handle_event(event)
        }
    }
}
