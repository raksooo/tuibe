use crate::{
    config::rss::RssConfigHandler,
    interface::component::{Component, EventSender, Frame, UpdateEvent},
};
use crossterm::event::{Event, KeyCode};
use tui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem},
};

pub struct RssConfigView {
    program_sender: EventSender,
    rss_config: RssConfigHandler,
    selected: usize,
}

impl RssConfigView {
    pub fn new(program_sender: EventSender, rss_config: RssConfigHandler) -> Self {
        Self {
            program_sender,
            rss_config,
            selected: 0,
        }
    }

    fn move_up(&mut self) -> UpdateEvent {
        if self.selected > 0 {
            self.selected -= 1;
        }
        UpdateEvent::Redraw
    }

    fn move_down(&mut self) -> UpdateEvent {
        if self.selected + 1 < self.rss_config.feeds().len() {
            self.selected += 1;
        }
        UpdateEvent::Redraw
    }

    fn remove_selected(&mut self) -> UpdateEvent {
        let url = self
            .rss_config
            .feeds()
            .get(self.selected)
            .unwrap()
            .url
            .to_string();

        let remove_receiver = self.rss_config.remove_feed(url);
        let program_sender = self.program_sender.clone();
        tokio::spawn(async move {
            remove_receiver.await.unwrap().unwrap();
            let _ = program_sender.send(UpdateEvent::Redraw).await;
        });
        UpdateEvent::None
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
        let list = self.create_list();
        f.render_widget(list, area);
    }

    fn handle_event(&mut self, event: Event) -> UpdateEvent {
        if let Event::Key(event) = event {
            match event.code {
                KeyCode::Char('d') => self.remove_selected(),
                KeyCode::Up => self.move_up(),
                KeyCode::Down => self.move_down(),
                KeyCode::Char('j') => self.move_down(),
                KeyCode::Char('k') => self.move_up(),
                _ => UpdateEvent::None,
            }
        } else {
            UpdateEvent::None
        }
    }
}
