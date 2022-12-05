use crate::{
    backend::{
        channel::BackendMessage,
        rss::{Feed, RssBackend},
    },
    interface::{
        component::{Component, Frame},
        list::{List, Same},
        status_label::{StatusLabelActions, LOADING_STRING},
    },
};

use crossterm::event::{Event, KeyCode};
use parking_lot::Mutex;
use std::sync::Arc;
use tui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, ListItem},
};

impl From<Feed> for ListItem<'static> {
    fn from(value: Feed) -> Self {
        ListItem::new(format!("  {}", value.title))
    }
}

impl Same for Feed {
    fn same(&self, other: &Self) -> bool {
        self.url == other.url
    }
}

pub struct RssBackendView {
    actions: StatusLabelActions,
    backend: Arc<RssBackend>,
    list: Arc<Mutex<List<Feed>>>,
}

impl RssBackendView {
    pub fn new(actions: StatusLabelActions, backend: Arc<RssBackend>) -> Self {
        let rss_backend_view = Self {
            actions,
            backend: backend.clone(),
            list: Arc::new(Mutex::new(List::new())),
        };

        rss_backend_view.listen_backend_messages(backend);
        rss_backend_view
    }

    fn listen_backend_messages(&self, backend: Arc<RssBackend>) {
        let actions = self.actions.clone();
        let list = self.list.clone();
        tokio::spawn(async move {
            let mut receiver = backend.subscribe_feeds();
            while let Some(message) = receiver.recv().await {
                Self::handle_backend_message(message, actions.clone(), list.clone()).await;
            }
        });
    }

    async fn handle_backend_message(
        message: BackendMessage<Feed>,
        actions: StatusLabelActions,
        list: Arc<Mutex<List<Feed>>>,
    ) {
        match message {
            BackendMessage::Error(_) => return, // Errors should be handled through feed_view
            BackendMessage::FinishedFetching => return, // Not necessary since there's no indicator
            BackendMessage::New(feed) => list.lock().add(feed),
            BackendMessage::Remove(feed) => list.lock().remove(&feed),
            BackendMessage::Clear => list.lock().clear(),
        }
        actions.redraw_async().await;
    }

    fn remove_selected(&mut self) {
        if let Some(feed) = self.list.lock().get_current_item() {
            let backend = self.backend.clone();
            let actions = self.actions.clone();

            tokio::spawn(async move {
                let remove_result = backend.remove_feed(&feed.url).await;
                actions.redraw_or_error_async(remove_result, true).await;
            });
        }
    }

    fn add_url(&self, url: &str) {
        let finish_loading = self.actions.show_label(LOADING_STRING);

        let url = url.to_owned();
        let backend = self.backend.clone();
        let actions = self.actions.clone();

        tokio::spawn(async move {
            let add_result = backend.add_feed(&url).await;
            finish_loading();
            actions.redraw_or_error_async(add_result, true).await;
        });
    }
}

impl Component for RssBackendView {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        let list = self.list.lock();
        let list = list.list(area.height.into());
        let styled_list = list
            .block(Block::default().title("Feeds").borders(Borders::RIGHT))
            .style(Style::default().fg(Color::White));
        f.render_widget(styled_list, area);
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Key(event) => match event.code {
                KeyCode::Char('d') => self.remove_selected(),
                KeyCode::Up => self.list.lock().move_up(),
                KeyCode::Down => self.list.lock().move_down(),
                KeyCode::Char('j') => self.list.lock().move_down(),
                KeyCode::Char('k') => self.list.lock().move_up(),
                _ => return,
            },
            Event::Paste(url) => self.add_url(&url),
            _ => return,
        }

        self.actions.redraw();
    }

    fn registered_events(&self) -> Vec<(String, String)> {
        vec![
            (String::from("j"), String::from("Down")),
            (String::from("k"), String::from("Up")),
            (String::from("Paste"), String::from("Add feed")),
        ]
    }
}
