use crate::{
    config::Config,
    feed::{Feed as VideoFeed, Video},
    interface::{
        component::{Component, EventSender, Frame, UpdateEvent},
        dialog,
        loading_indicator::LoadingIndicator,
    },
};
use crossterm::event::{Event, KeyCode};
use std::sync::{Arc, Mutex};
use tui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

pub struct Feed {
    config: Option<Config>,
    videos: Arc<Mutex<Result<Option<Vec<Video>>, ()>>>,
    current_item: Arc<Mutex<usize>>,

    tx: EventSender,
    loading_indicator: LoadingIndicator,
}

impl Feed {
    pub fn new(tx: EventSender, config: Option<Config>) -> Self {
        let videos = Arc::new(Mutex::new(Ok(None)));
        let current_item = Arc::new(Mutex::new(0));
        let loading_indicator = LoadingIndicator::new(tx.clone());

        let mut new_feed = Self {
            config,
            videos,
            current_item,
            tx,
            loading_indicator,
        };

        new_feed.reload();
        new_feed
    }

    pub fn update_with_config(&mut self, config: &Config) {
        self.config = Some(config.to_owned());
        self.reload();
    }

    fn reload(&mut self) {
        let tx = self.tx.clone();
        let videos = Arc::clone(&self.videos);
        let current_item = Arc::clone(&self.current_item);

        if let Some(config) = &self.config {
            let config = config.clone();

            tokio::spawn(async move {
                {
                    let mut videos = videos.lock().unwrap();
                    *videos = Ok(None);
                    let mut current_item = current_item.lock().unwrap();
                    *current_item = 0;
                }

                let _ = tx.send(UpdateEvent::Redraw).await;

                {
                    let new_videos = VideoFeed::from_config(&config).await;
                    let mut videos = videos.lock().unwrap();
                    match new_videos {
                        Ok(feed) if feed.videos.len() > 0 => *videos = Ok(Some(feed.videos)),
                        _ => *videos = Err(()),
                    }
                }

                let _ = tx.send(UpdateEvent::Redraw).await;
            });
        }
    }

    fn toggle_current_item(&self) {
        let tx = self.tx.clone();
        let videos = Arc::clone(&self.videos);
        let current_item = Arc::clone(&self.current_item);

        tokio::spawn(async move {
            {
                let current_item = current_item.lock().unwrap();
                if let Ok(Some(ref mut videos)) = *videos.lock().unwrap() {
                    if let Some(video) = videos.get_mut(*current_item) {
                        video.toggle_selected();
                    }
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
                if let Ok(Some(ref videos)) = *videos.lock().unwrap() {
                    let mut current_item = current_item.lock().unwrap();
                    *current_item = std::cmp::min(*current_item + 1, videos.len() - 1);
                }
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
        if let Ok(ref videos) = *self.videos.lock().unwrap() {
            if let Some(videos) = videos {
                let description_height = 10;
                let description_y = size.height - description_height;
                let list_size = Rect::new(0, 0, size.width, description_y - 10);
                let description_size = Rect::new(0, description_y, size.width, description_height);

                let current_item = *self.current_item.lock().unwrap();
                let list = Self::create_list(&videos, current_item, list_size.width.into());
                let description = Self::create_description(&videos, current_item);

                f.render_widget(list, list_size);
                f.render_widget(description, description_size);
            } else {
                self.loading_indicator.draw(f, size);
            }
        } else {
            dialog::dialog(f, size, "Something went wrong..");
        }
    }

    fn handle_event(&mut self, event: Event) {
        if let Event::Key(event) = event {
            match event.code {
                KeyCode::Char(' ') => self.toggle_current_item(),
                KeyCode::Up => self.move_up(),
                KeyCode::Down => self.move_down(),
                KeyCode::Char('j') => self.move_down(),
                KeyCode::Char('k') => self.move_up(),
                KeyCode::Char('r') => self.reload(),
                _ => (),
            }
        }
    }
}
