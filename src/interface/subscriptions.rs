use crate::interface::component::{Component, EventSender, Frame, UpdateEvent};
use crossterm::event::{Event, KeyCode};
use std::collections::HashMap;
use tui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem},
};

pub struct Subscriptions {
    tx: EventSender,
    channels: HashMap<String, String>,
    selected: usize,
}

impl Subscriptions {
    pub fn new(tx: EventSender, channels: HashMap<String, String>) -> Self {
        // TODO: Handle empty map
        Subscriptions {
            tx,
            channels,
            selected: 0,
        }
    }

    pub fn update_channels(&mut self, channels: HashMap<String, String>) {
        self.channels = channels;

        let tx = self.tx.clone();
        tokio::spawn(async move {
            let _ = tx.send(UpdateEvent::Redraw).await;
        });
    }

    fn move_up(&mut self) -> UpdateEvent {
        if self.selected > 0 {
            self.selected = self.selected - 1;
        }
        UpdateEvent::Redraw
    }

    fn move_down(&mut self) -> UpdateEvent {
        self.selected = std::cmp::min(self.selected + 1, self.channels.len() - 1);
        UpdateEvent::Redraw
    }

    fn delete_selected(&mut self) -> UpdateEvent {
        // TODO
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
    fn draw(&mut self, f: &mut Frame, size: Rect) {
        let list = self.create_list();
        f.render_widget(list, size);
    }

    fn handle_event(&mut self, event: Event) -> UpdateEvent {
        if let Event::Key(event) = event {
            match event.code {
                KeyCode::Char('d') => self.delete_selected(),
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
