use super::{
    component::{Component, Frame},
    error_handler::ErrorHandlerActions,
    feed_view::FeedView,
};
use crate::config::{common::CommonConfigHandler, Config};

use crossterm::event::{Event, KeyCode};
use parking_lot::Mutex;
use std::sync::Arc;
use tui::layout::{Constraint, Direction, Layout, Rect};

pub struct MainView {
    actions: ErrorHandlerActions,

    show_config: Arc<Mutex<bool>>,

    feed: FeedView,
    config: Box<dyn Component + Send>,
}

impl MainView {
    pub fn new<C, CF>(
        actions: ErrorHandlerActions,
        common_config: CommonConfigHandler,
        config: Arc<impl Config + Send + Sync + 'static>,
        config_creator: CF,
    ) -> Self
    where
        C: Component + Send + 'static,
        CF: FnOnce(ErrorHandlerActions) -> C,
    {
        Self {
            show_config: Arc::new(Mutex::new(false)),
            feed: FeedView::new(actions.clone(), common_config, config),
            config: Box::new(config_creator(actions.clone())),
            actions,
        }
    }
}

impl Component for MainView {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        let show_config = self.show_config.lock();
        let config_numerator = u32::from(*show_config);

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

        if *show_config {
            self.config.draw(f, chunks[0]);
        }

        self.feed.draw(f, chunks[1]);
    }

    fn handle_event(&mut self, event: Event) {
        if *self.show_config.lock() {
            if matches!(event, Event::Key(event) if event.code == KeyCode::Esc) {
                let mut show_config = self.show_config.lock();
                *show_config = false;
                self.actions.redraw();
            } else {
                self.config.handle_event(event);
            }
        } else if matches!(event, Event::Key(event) if event.code == KeyCode::Char('c')) {
            {
                let mut show_config = self.show_config.lock();
                *show_config = true;
            }
            self.actions.redraw();
        } else {
            self.feed.handle_event(event);
        }
    }

    fn registered_events(&self) -> Vec<(String, String)> {
        if *self.show_config.lock() {
            let mut events = vec![(String::from("Esc"), String::from("Back"))];
            events.append(&mut self.config.registered_events());
            events
        } else {
            let mut events = vec![(String::from("c"), String::from("Configure"))];
            events.append(&mut self.feed.registered_events());
            events
        }
    }
}
