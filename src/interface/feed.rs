use crate::{
    interface::component::{Component, EventSender, Frame, UpdateEvent},
    video::Video,
};
use crossterm::event::{Event, KeyCode};
use tui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

pub struct Feed {
    videos: Vec<Video>,
    current_item: usize,
}

impl Feed {
    pub fn new(_tx: EventSender, mut videos: Vec<Video>) -> Self {
        videos.reverse();
        Self {
            videos,
            current_item: 0,
        }
    }

    fn toggle_current_item(&mut self) -> UpdateEvent {
        if let Some(video) = self.videos.get_mut(self.current_item) {
            video.toggle_selected();
        }
        UpdateEvent::Redraw
    }

    fn move_up(&mut self) -> UpdateEvent {
        if self.current_item > 0 {
            self.current_item = self.current_item - 1;
        }
        UpdateEvent::Redraw
    }

    fn move_down(&mut self) -> UpdateEvent {
        self.current_item = std::cmp::min(self.current_item + 1, self.videos.len() - 1);
        UpdateEvent::Redraw
    }

    fn create_list(&self, width: usize) -> List<'_> {
        let mut items: Vec<ListItem> = Vec::new();

        for (i, video) in self.videos.iter().enumerate() {
            let mut item = ListItem::new(video.get_label(width));
            if i == self.current_item {
                item = item.style(Style::default().fg(Color::Green));
            }
            items.push(item);
        }

        List::new(items)
            .block(Block::default().title("Videos"))
            .style(Style::default().fg(Color::White))
    }

    fn create_description(&self) -> Paragraph<'_> {
        // current_item is always within the bounds of videos
        let description = self
            .videos
            .get(self.current_item)
            .unwrap()
            .description
            .to_owned();

        Paragraph::new(description)
            .block(Block::default().title("Description").borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true })
    }
}

impl Component for Feed {
    fn draw(&mut self, f: &mut Frame, size: Rect) {
        let description_height = 10;
        let description_y = size.height - description_height;
        let list_size = Rect::new(size.x, 0, size.width, description_y - 10);
        let description_size = Rect::new(size.x, description_y, size.width, description_height);

        let list = self.create_list(list_size.width.into());
        let description = self.create_description();

        f.render_widget(list, list_size);
        f.render_widget(description, description_size);
    }

    fn handle_event(&mut self, event: Event) -> UpdateEvent {
        if let Event::Key(event) = event {
            match event.code {
                KeyCode::Char(' ') => self.toggle_current_item(),
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
