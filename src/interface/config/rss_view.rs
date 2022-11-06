use crate::{
    config::rss::RssConfigHandler,
    interface::{
        component::{Component, Frame},
        error_handler::{ErrorMsg, ErrorSenderExt},
        loading_indicator::LoadingIndicator,
        main_view::MainViewMsg,
    },
};
use crossterm::event::{Event, KeyCode};
use parking_lot::Mutex;
use std::sync::Arc;
use tui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem},
};

pub struct RssConfigView {
    redraw_sender: flume::Sender<()>,
    error_sender: flume::Sender<ErrorMsg>,
    main_sender: flume::Sender<MainViewMsg>,
    rss_config: RssConfigHandler,
    selected: usize,
    loading_indicator: Arc<Mutex<Option<LoadingIndicator>>>,
}

impl RssConfigView {
    pub fn new(
        redraw_sender: flume::Sender<()>,
        error_sender: flume::Sender<ErrorMsg>,
        main_sender: flume::Sender<MainViewMsg>,
        rss_config: RssConfigHandler,
    ) -> Self {
        Self {
            redraw_sender,
            error_sender,
            main_sender,
            rss_config,
            selected: 0,
            loading_indicator: Arc::new(Mutex::new(None)),
        }
    }

    fn close(&self) {
        let _ = self.main_sender.send(MainViewMsg::CloseConfig);
    }

    fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            let _ = self.redraw_sender.send(());
        }
    }

    fn move_down(&mut self) {
        if self.selected + 1 < self.rss_config.feeds().len() {
            self.selected += 1;
            let _ = self.redraw_sender.send(());
        }
    }

    fn remove_selected(&mut self) {
        let url = self
            .rss_config
            .feeds()
            .get(self.selected)
            .unwrap()
            .url
            .clone();

        let remove_receiver = self.rss_config.remove_feed(&url);
        let redraw_sender = self.redraw_sender.clone();
        let error_sender = self.error_sender.clone();

        tokio::spawn(async move {
            let remove_result = remove_receiver.await.unwrap();
            error_sender
                .run_or_send_async(remove_result, true, |_| async {
                    let _ = redraw_sender.send_async(()).await;
                })
                .await;
        });
    }

    fn add_url(&self, url: &str) {
        {
            let mut loading_indicator = self.loading_indicator.lock();
            *loading_indicator = Some(LoadingIndicator::new(self.redraw_sender.clone()));
        }
        let _ = self.redraw_sender.send(());

        let add_receiver = self.rss_config.add_feed(url);
        let redraw_sender = self.redraw_sender.clone();
        let error_sender = self.error_sender.clone();
        let loading_indicator = Arc::clone(&self.loading_indicator);

        tokio::spawn(async move {
            {
                let mut loading_indicator = loading_indicator.lock();
                *loading_indicator = None;
            }

            let add_result = add_receiver.await.unwrap();
            error_sender
                .run_or_send_async(add_result, true, |_| async {
                    let _ = redraw_sender.send_async(()).await;
                })
                .await;
        });
    }

    fn create_list(&self, area: Rect) -> List<'_> {
        let mut items: Vec<ListItem> = Vec::new();
        let feeds = self.rss_config.feeds();

        let height: usize = area.height.into();
        let nfeeds = feeds.len();
        let start_index = if self.selected < height / 2 {
            0
        } else if self.selected >= nfeeds - height / 2 {
            nfeeds - height + 1
        } else {
            self.selected - (height / 2)
        };

        for (index, feed) in feeds.iter().skip(start_index).enumerate() {
            let mut item = ListItem::new(feed.title.clone());
            if index + start_index == self.selected {
                item = item.style(Style::default().fg(Color::Green));
            }
            items.push(item);
        }

        List::new(items)
            .block(Block::default().title("Feeds").borders(Borders::RIGHT))
            .style(Style::default().fg(Color::White))
    }
}

impl Component for RssConfigView {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        let list = self.create_list(area);
        f.render_widget(list, area);

        if let Some(ref mut loading_indicator) = *self.loading_indicator.lock() {
            loading_indicator.draw(f, area);
        }
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Key(event) => match event.code {
                KeyCode::Esc => self.close(),
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
            (String::from("Esc"), String::from("Close")),
            (String::from("j"), String::from("Down")),
            (String::from("k"), String::from("Up")),
            (String::from("Paste"), String::from("Add feed")),
        ]
    }
}
