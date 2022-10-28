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
use tokio::sync::mpsc;
use tui::layout::{Constraint, Direction, Layout, Rect};

pub struct App {
    show_config: bool,

    feed: FeedView,
    config: Box<dyn Component + Send>,

    config_sender: mpsc::Sender<ConfigProviderMsg>,
}

impl App {
    pub fn new(
        config_sender: mpsc::Sender<ConfigProviderMsg>,
        common_config: CommonConfigHandler,
        videos: Vec<Video>,
        config: impl Component + Send + 'static,
    ) -> Self {
        let last_played_timestamp = common_config.config().last_played_timestamp;

        Self {
            show_config: false,

            feed: FeedView::new(videos, last_played_timestamp),
            config: Box::new(config),

            config_sender,
        }
    }

    fn toggle_config(&mut self) -> UpdateEvent {
        self.show_config = !self.show_config;
        if !self.show_config {
            self.config_sender.send_sync(ConfigProviderMsg::Reload);
        }
        UpdateEvent::Redraw
    }
}

impl Component for App {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        let config_numerator = if self.show_config { 1 } else { 0 };

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

        if self.show_config {
            self.config.draw(f, chunks[0]);
        }

        self.feed.draw(f, chunks[1]);
    }

    fn handle_event(&mut self, event: Event) -> UpdateEvent {
        if let Event::Key(event) = event {
            match event.code {
                KeyCode::Char('q') => return UpdateEvent::Quit,
                KeyCode::Char('s') => return self.toggle_config(),
                _ => (),
            }
        }

        if self.show_config {
            self.config.handle_event(event)
        } else {
            self.feed.handle_event(event)
        }
    }
}
