use crate::{
    config::{
        config_message_channel::ConfigMessage,
        rss::{Feed, RssConfigHandler},
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

pub struct RssConfigView {
    actions: StatusLabelActions,
    rss_config: Arc<RssConfigHandler>,
    list: Arc<Mutex<List<Feed>>>,
}

impl RssConfigView {
    pub fn new(actions: StatusLabelActions, rss_config: Arc<RssConfigHandler>) -> Self {
        let rss_config_view = Self {
            actions,
            rss_config: rss_config.clone(),
            list: Arc::new(Mutex::new(List::new())),
        };

        rss_config_view.listen_config_messages(rss_config);
        rss_config_view
    }

    fn listen_config_messages(&self, config: Arc<RssConfigHandler>) {
        let actions = self.actions.clone();
        let list = self.list.clone();
        tokio::spawn(async move {
            let mut receiver = config.subscribe_feeds();
            while let Some(message) = receiver.recv().await {
                Self::handle_config_message(message, actions.clone(), list.clone()).await;
            }
        });
    }

    async fn handle_config_message(
        message: ConfigMessage<Feed>,
        actions: StatusLabelActions,
        list: Arc<Mutex<List<Feed>>>,
    ) {
        match message {
            ConfigMessage::Error(_) => return, // Errors should be handled through feed_view
            ConfigMessage::FinishedFetching => return, // Not necessary since there's no indicator
            ConfigMessage::New(feed) => list.lock().add(feed),
            ConfigMessage::Remove(feed) => list.lock().remove(&feed),
            ConfigMessage::Clear => list.lock().clear(),
        }
        actions.redraw_async().await;
    }

    fn remove_selected(&mut self) {
        if let Some(feed) = self.list.lock().get_current_item() {
            let rss_config = self.rss_config.clone();
            let actions = self.actions.clone();

            tokio::spawn(async move {
                let remove_result = rss_config.remove_feed(&feed.url).await;
                actions.redraw_or_error_async(remove_result, true).await;
            });
        }
    }

    fn add_url(&self, url: &str) {
        let finish_loading = self.actions.show_label(LOADING_STRING);

        let url = url.to_owned();
        let rss_config = self.rss_config.clone();
        let actions = self.actions.clone();

        tokio::spawn(async move {
            let add_result = rss_config.add_feed(&url).await;
            finish_loading();
            actions.redraw_or_error_async(add_result, true).await;
        });
    }
}

impl Component for RssConfigView {
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
