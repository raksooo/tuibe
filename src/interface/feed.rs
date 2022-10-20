use crate::{
    interface::component::{Component, EventSender, Frame, UpdateEvent},
    video::Video,
};
use crossterm::event::{Event, KeyCode};
use std::sync::{Arc, Mutex};
use tui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

pub struct Feed {
    videos: Arc<Mutex<Vec<Video>>>,
    current_item: Arc<Mutex<usize>>,

    tx: EventSender,
}

impl Feed {
    pub fn new(tx: EventSender, mut videos: Vec<Video>) -> Self {
        videos.reverse();
        Self {
            videos: Arc::new(Mutex::new(videos)),
            current_item: Arc::new(Mutex::new(0)),
            tx,
        }
    }

    fn toggle_current_item(&self) {
        let tx = self.tx.clone();
        let videos = Arc::clone(&self.videos);
        let current_item = Arc::clone(&self.current_item);

        tokio::spawn(async move {
            {
                let current_item = current_item.lock().unwrap();
                let mut videos = videos.lock().unwrap();
                if let Some(video) = videos.get_mut(*current_item) {
                    video.toggle_selected();
                }
            }

            let _ = tx.send(UpdateEvent::Redraw).await;
        });
    }

    fn move_up(&mut self) {
        let tx = self.tx.clone();
        let current_item = Arc::clone(&self.current_item);

        tokio::spawn(async move {
            {
                let mut current_item = current_item.lock().unwrap();
                if *current_item > 0 {
                    *current_item = *current_item - 1;
                }
            }

            let _ = tx.send(UpdateEvent::Redraw).await;
        });
    }

    fn move_down(&mut self) {
        let tx = self.tx.clone();
        let videos = Arc::clone(&self.videos);
        let current_item = Arc::clone(&self.current_item);

        tokio::spawn(async move {
            {
                let videos = videos.lock().unwrap();
                let mut current_item = current_item.lock().unwrap();
                *current_item = std::cmp::min(*current_item + 1, videos.len() - 1);
            }

            let _ = tx.send(UpdateEvent::Redraw).await;
        });
    }

    fn create_list(videos: &Vec<Video>, current_item: usize, width: usize) -> List<'_> {
        let mut items: Vec<ListItem> = Vec::new();

        for (i, video) in videos.iter().enumerate() {
            let mut item = ListItem::new(video.get_label(width));
            if i == current_item {
                item = item.style(Style::default().fg(Color::Green));
            }
            items.push(item);
        }

        List::new(items)
            .block(Block::default().title("Videos"))
            .style(Style::default().fg(Color::White))
    }

    fn create_description(videos: &Vec<Video>, current_item: usize) -> Paragraph<'_> {
        // current_item is always within the bounds of videos
        let description = videos.get(current_item).unwrap().description.to_owned();

        Paragraph::new(description)
            .block(Block::default().title("Description").borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true })
    }
}

impl Component for Feed {
    fn draw(&mut self, f: &mut Frame, size: Rect) {
        let videos = self.videos.lock().unwrap();

        let description_height = 10;
        let description_y = size.height - description_height;
        let list_size = Rect::new(0, 0, size.width, description_y - 10);
        let description_size = Rect::new(0, description_y, size.width, description_height);

        let current_item = *self.current_item.lock().unwrap();
        let list = Self::create_list(&videos, current_item, list_size.width.into());
        let description = Self::create_description(&videos, current_item);

        f.render_widget(list, list_size);
        f.render_widget(description, description_size);
    }

    fn handle_event(&mut self, event: Event) {
        if let Event::Key(event) = event {
            match event.code {
                KeyCode::Char(' ') => self.toggle_current_item(),
                KeyCode::Up => self.move_up(),
                KeyCode::Down => self.move_down(),
                KeyCode::Char('j') => self.move_down(),
                KeyCode::Char('k') => self.move_up(),
                _ => (),
            }
        }
    }
}
