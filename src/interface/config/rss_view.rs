use crate::{
    config::{
        config_message_channel::ConfigMessage,
        rss::{Feed, RssConfigHandler},
    },
    interface::{
        component::{Component, Frame},
        list::generate_items,
        status_label::{StatusLabelActions, LOADING_STRING},
    },
};

use crossterm::event::{Event, KeyCode};
use parking_lot::Mutex;
use std::sync::Arc;
use tui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List},
};

pub struct RssConfigView {
    actions: StatusLabelActions,
    rss_config: Arc<RssConfigHandler>,
    feeds: Arc<Mutex<Vec<Feed>>>,
    selected: usize,
}

impl RssConfigView {
    pub fn new(actions: StatusLabelActions, rss_config: Arc<RssConfigHandler>) -> Self {
        let rss_config_view = Self {
            actions,
            rss_config: rss_config.clone(),
            feeds: Arc::new(Mutex::new(Vec::new())),
            selected: 0,
        };

        rss_config_view.listen_config_messages(rss_config);
        rss_config_view
    }

    fn listen_config_messages(&self, config: Arc<RssConfigHandler>) {
        let actions = self.actions.clone();
        let feeds = self.feeds.clone();
        tokio::spawn(async move {
            let mut receiver = config.subscribe_feeds();
            while let Some(message) = receiver.recv().await {
                Self::handle_config_message(message, actions.clone(), feeds.clone()).await;
            }
        });
    }

    async fn handle_config_message(
        message: ConfigMessage<Feed>,
        actions: StatusLabelActions,
        feeds: Arc<Mutex<Vec<Feed>>>,
    ) {
        match message {
            ConfigMessage::Error(_) => (), // Errors should be handled through feed_view
            ConfigMessage::FinishedFetching => (), // Not necessary since there's no indicator
            ConfigMessage::New(feed) => {
                let mut feeds = feeds.lock();
                feeds.push(feed);
            }
            ConfigMessage::Remove(feed) => {
                let mut feeds = feeds.lock();
                feeds.retain(|current| current != &feed);
            }
            ConfigMessage::Clear => {
                let mut feeds = feeds.lock();
                feeds.clear();
            }
        }
        actions.redraw_async().await;
    }

    fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.actions.redraw();
        }
    }

    fn move_down(&mut self) {
        let feeds = self.feeds.lock();
        if self.selected + 1 < feeds.len() {
            self.selected += 1;
            self.actions.redraw();
        }
    }

    fn remove_selected(&mut self) {
        // selected is always within the bounds of feeds
        let feeds = self.feeds.lock();
        let url = feeds.get(self.selected).unwrap().url.clone();

        let rss_config = self.rss_config.clone();
        let actions = self.actions.clone();

        tokio::spawn(async move {
            let remove_result = rss_config.remove_feed(&url).await;
            actions.redraw_or_error_async(remove_result, true).await;
        });
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

    fn create_list(&self, area: Rect) -> List<'_> {
        let feeds = self.feeds.lock().to_vec();
        let items = generate_items(area, self.selected, feeds, |feed| feed.title);
        List::new(items)
            .block(Block::default().title("Feeds").borders(Borders::RIGHT))
            .style(Style::default().fg(Color::White))
    }
}

impl Component for RssConfigView {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        let list = self.create_list(area);
        f.render_widget(list, area);
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Key(event) => match event.code {
                KeyCode::Char('d') => self.remove_selected(),
                KeyCode::Up => self.move_up(),
                KeyCode::Down => self.move_down(),
                KeyCode::Char('j') => self.move_down(),
                KeyCode::Char('k') => self.move_up(),
                _ => (),
            },
            Event::Paste(url) => self.add_url(&url),
            _ => (),
        }
    }

    fn registered_events(&self) -> Vec<(String, String)> {
        vec![
            (String::from("j"), String::from("Down")),
            (String::from("k"), String::from("Up")),
            (String::from("Paste"), String::from("Add feed")),
        ]
    }
}
