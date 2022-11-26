use crate::{
    config::rss::{Feed, RssConfigHandler},
    interface::{
        component::{Component, Frame},
        list::generate_items,
        status_label::{StatusLabelActions, LOADING_STRING},
    },
};

use crossterm::event::{Event, KeyCode};
use std::sync::Arc;
use tui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List},
};

pub struct RssConfigView {
    actions: StatusLabelActions,
    rss_config: Arc<RssConfigHandler>,
    selected: usize,
}

impl RssConfigView {
    pub fn new(actions: StatusLabelActions, rss_config: Arc<RssConfigHandler>) -> Self {
        Self {
            actions,
            rss_config,
            selected: 0,
        }
    }

    fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.actions.redraw();
        }
    }

    fn move_down(&mut self) {
        if self.selected + 1 < self.rss_config.feeds().len() {
            self.selected += 1;
            self.actions.redraw();
        }
    }

    fn remove_selected(&mut self) {
        // selected is always within the bounds of feeds
        let url = self.feeds().get(self.selected).unwrap().url.clone();

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
        let feeds = self.feeds();
        let items = generate_items(area, self.selected, feeds, |feed| feed.title);
        List::new(items)
            .block(Block::default().title("Feeds").borders(Borders::RIGHT))
            .style(Style::default().fg(Color::White))
    }

    fn feeds(&self) -> Vec<Feed> {
        let mut feeds = self.rss_config.feeds();
        feeds.sort();
        feeds
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
