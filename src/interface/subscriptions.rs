use super::{
    app::AppMsg,
    component::{Component, EventSender, Frame, UpdateEvent},
};
use crate::sender_ext::SenderExt;
use crossterm::event::{Event, KeyCode};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem},
};

pub struct Subscriptions {
    program_tx: EventSender,
    app_tx: mpsc::Sender<AppMsg>,
    channels: HashMap<String, String>,
    selected: usize,
}

impl Subscriptions {
    pub fn new(
        program_tx: EventSender,
        app_tx: mpsc::Sender<AppMsg>,
        channels: HashMap<String, String>,
    ) -> Self {
        Subscriptions {
            program_tx,
            app_tx,
            channels,
            selected: 0,
        }
    }

    pub fn update_channels(&mut self, channels: HashMap<String, String>) {
        self.channels = channels;
        self.program_tx.send_sync(UpdateEvent::Redraw);
    }

    fn move_up(&mut self) -> UpdateEvent {
        if self.selected > 0 {
            self.selected -= 1;
        }
        UpdateEvent::Redraw
    }

    fn move_down(&mut self) -> UpdateEvent {
        if self.selected + 1 < self.channels.len() {
            self.selected += 1;
        }
        UpdateEvent::Redraw
    }

    fn remove_selected(&mut self) -> UpdateEvent {
        let subscription = self.channels.keys().nth(self.selected).unwrap().to_string();
        self.app_tx
            .send_sync(AppMsg::RemoveSubscription(subscription));
        UpdateEvent::None
    }

    fn create_list(&self) -> List<'_> {
        let mut items: Vec<ListItem> = Vec::new();

        for (index, title) in self.channels.values().enumerate() {
            let mut item = ListItem::new(title.to_string());
            if index == self.selected {
                item = item.style(Style::default().fg(Color::Green));
            }
            items.push(item);
        }

        List::new(items)
            .block(Block::default().title("Channels").borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
    }
}

impl Component for Subscriptions {
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
