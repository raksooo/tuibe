use crate::{
    config::rss::RssConfigHandler,
    interface::{
        component::{Component, EventSender, Frame, UpdateEvent},
        error_handler::{ErrorMsg, ErrorSenderExt},
        loading_indicator::LoadingIndicator,
        main_view::MainViewMsg,
    },
    sender_ext::SenderExt,
};
use crossterm::event::{Event, KeyCode};
use parking_lot::Mutex;
use std::sync::Arc;
use tokio::sync::mpsc;
use tui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub struct RssConfigView {
    program_sender: EventSender,
    error_sender: mpsc::Sender<ErrorMsg>,
    main_sender: mpsc::Sender<MainViewMsg>,
    rss_config: RssConfigHandler,
    selected: usize,
    loading_indicator: Arc<Mutex<Option<LoadingIndicator>>>,
}

impl RssConfigView {
    pub fn new(
        program_sender: EventSender,
        error_sender: mpsc::Sender<ErrorMsg>,
        main_sender: mpsc::Sender<MainViewMsg>,
        rss_config: RssConfigHandler,
    ) -> Self {
        Self {
            program_sender,
            error_sender,
            main_sender,
            rss_config,
            selected: 0,
            loading_indicator: Arc::new(Mutex::new(None)),
        }
    }

    fn close(&self) {
        self.main_sender.send_sync(MainViewMsg::CloseConfig);
    }

    fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.program_sender.send_sync(UpdateEvent::Redraw);
        }
    }

    fn move_down(&mut self) {
        if self.selected + 1 < self.rss_config.feeds().len() {
            self.selected += 1;
            self.program_sender.send_sync(UpdateEvent::Redraw);
        }
    }

    fn remove_selected(&mut self) {
        let url = self
            .rss_config
            .feeds()
            .get(self.selected)
            .unwrap()
            .url
            .to_string();

        let remove_receiver = self.rss_config.remove_feed(url);
        let program_sender = self.program_sender.clone();
        let error_sender = self.error_sender.clone();
        tokio::spawn(async move {
            let remove_result = remove_receiver.await.unwrap();
            error_sender
                .run_or_send_async(remove_result, true, |_| async {
                    let _ = program_sender.send(UpdateEvent::Redraw).await;
                })
                .await;
        });
    }

    fn add_url(&self, url: String) {
        {
            let mut loading_indicator = self.loading_indicator.lock();
            *loading_indicator = Some(LoadingIndicator::new(self.program_sender.clone()));
        }
        self.program_sender.send_sync(UpdateEvent::Redraw);

        let add_receiver = self.rss_config.add_feed(url);
        let program_sender = self.program_sender.clone();
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
                    let _ = program_sender.send(UpdateEvent::Redraw).await;
                })
                .await;
        });
    }

    fn create_list(&self) -> List<'_> {
        let mut items: Vec<ListItem> = Vec::new();

        for (index, feed) in self.rss_config.feeds().iter().enumerate() {
            let mut item = ListItem::new(feed.title.to_string());
            if index == self.selected {
                item = item.style(Style::default().fg(Color::Green));
            }
            items.push(item);
        }

        List::new(items)
            .block(Block::default().title("Feeds").borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
    }
}

impl Component for RssConfigView {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        let instruction_height = 2;
        let list_area = Rect::new(area.x, area.y, area.width, area.height - instruction_height);
        let instruction_area = Rect::new(
            area.x,
            area.height - instruction_height,
            area.width,
            instruction_height,
        );

        let list = self.create_list();
        f.render_widget(list, list_area);

        let instruction = Paragraph::new("Paste URL to add")
            .block(Block::default().borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM))
            .style(Style::default().fg(Color::White));
        f.render_widget(instruction, instruction_area);

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
            Event::Paste(url) => self.add_url(url),
            _ => (),
        }
    }
}
