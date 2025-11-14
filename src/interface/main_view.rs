use super::{
    actions::Actions,
    backend::rss_view::RssBackendView,
    component::{Component, Frame},
    feed_view::FeedView,
};
use crate::backend::rss::RssBackend;
use crate::config::ConfigHandler;

use crossterm::event::{Event, KeyCode};
use parking_lot::Mutex;
use ratatui::layout::{Constraint, Direction, Layout, Rect, Size};
use std::sync::Arc;

pub struct MainView {
    actions: Actions,

    show_backend_view: Arc<Mutex<bool>>,

    feed: FeedView,
    backend_view: RssBackendView,
}

impl MainView {
    pub fn new(actions: Actions, config: ConfigHandler, backend: Arc<RssBackend>) -> Self {
        Self {
            show_backend_view: Arc::new(Mutex::new(false)),
            feed: FeedView::new(actions.clone(), config, backend.clone()),
            backend_view: RssBackendView::new(actions.clone(), backend),
            actions,
        }
    }
}

impl Component for MainView {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        let show_backend_view = self.show_backend_view.lock();
        let backend_view_numerator = u32::from(*show_backend_view);

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Ratio(backend_view_numerator, 2),
                    Constraint::Ratio(2 - backend_view_numerator, 2),
                ]
                .as_ref(),
            )
            .split(area);

        if *show_backend_view {
            self.backend_view.draw(f, chunks[0]);
        }

        self.feed.draw(f, chunks[1]);
    }

    fn handle_event(&mut self, event: Event, size: Option<Size>) {
        if *self.show_backend_view.lock() {
            if matches!(event, Event::Key(event) if event.code == KeyCode::Esc) {
                let mut show_backend_view = self.show_backend_view.lock();
                *show_backend_view = false;
                self.actions.redraw();
            } else {
                self.backend_view.handle_event(event, size);
            }
        } else if matches!(event, Event::Key(event) if event.code == KeyCode::Char('c')) {
            {
                let mut show_backend_view = self.show_backend_view.lock();
                *show_backend_view = true;
            }
            self.actions.redraw();
        } else {
            self.feed.handle_event(event, size);
        }
    }

    fn registered_events(&self) -> Vec<(String, String)> {
        if *self.show_backend_view.lock() {
            let mut events = vec![(String::from("Esc"), String::from("Back"))];
            events.append(&mut self.backend_view.registered_events());
            events
        } else {
            let mut events = vec![(String::from("c"), String::from("Configure"))];
            events.append(&mut self.feed.registered_events());
            events
        }
    }
}
