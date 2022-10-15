use crate::config::Config;
use crate::feed::{Feed as VideoFeed, Video};
use crate::interface::{
    component::{handled_event, Component, EventFuture, Frame, UpdateEvent, UpdateSender},
    loading_indicator::LoadingIndicator,
};
use crossterm::event::{Event, KeyCode};
use std::sync::Arc;
use tokio::sync::Mutex;
use tui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

pub struct Feed {
    config: Option<Config>,
    videos: Arc<Mutex<Option<Vec<Video>>>>,
    current_item: Arc<Mutex<usize>>,

    pub loading_indicator: LoadingIndicator,
}

impl Feed {
    pub fn new(tx: UpdateSender, config: Option<Config>) -> Self {
        let new_feed = Self::create_empty(tx.clone(), config.clone());

        if let Some(config) = config {
            let videos = Arc::clone(&new_feed.videos);
            let current_item = Arc::clone(&new_feed.current_item);
            tokio::spawn(async move {
                Self::initiate_load_feed(videos, current_item, &config).await;
                tx.send(UpdateEvent::Redraw).await;
            });
        }

        new_feed
    }

    fn create_empty(_tx: UpdateSender, config: Option<Config>) -> Self {
        let videos = Arc::new(Mutex::new(None));
        let current_item = Arc::new(Mutex::new(0));

        let loading_indicator = LoadingIndicator::new();

        Self {
            config,
            videos,
            current_item,

            loading_indicator,
        }
    }

    pub async fn update_with_config(&mut self, config: &Config) {
        self.config = Some(config.to_owned());
        self.reload().await
    }

    fn reload(&mut self) -> EventFuture {
        if let Some(config) = &self.config {
            let videos = Arc::clone(&self.videos);
            let current_item = Arc::clone(&self.current_item);
            Self::initiate_load_feed(videos, current_item, &config)
        } else {
            handled_event()
        }
    }

    fn toggle_current_item(&self) -> EventFuture {
        let videos = Arc::clone(&self.videos);
        let current_item = Arc::clone(&self.current_item);
        Box::pin(async move {
            let current_item = current_item.lock().await;
            if let Some(ref mut videos) = *videos.lock().await {
                if let Some(video) = videos.get_mut(*current_item) {
                    video.toggle_selected();
                }
            }
        })
    }

    fn move_up(&mut self) -> EventFuture {
        let current_item = Arc::clone(&self.current_item);
        Box::pin(async move {
            let mut current_item = current_item.lock().await;
            if *current_item > 0 {
                *current_item = *current_item - 1;
            }
        })
    }

    fn move_down(&mut self) -> EventFuture {
        let videos = Arc::clone(&self.videos);
        let current_item = Arc::clone(&self.current_item);
        Box::pin(async move {
            if let Some(ref videos) = *videos.lock().await {
                let mut current_item = current_item.lock().await;
                *current_item = std::cmp::min(*current_item + 1, videos.len() - 1);
            }
        })
    }

    fn initiate_load_feed(
        videos: Arc<Mutex<Option<Vec<Video>>>>,
        current_item: Arc<Mutex<usize>>,
        config: &Config,
    ) -> EventFuture {
        let config = config.to_owned();
        Box::pin(async move {
            {
                let mut videos = videos.lock().await;
                *videos = None;
                let mut current_item = current_item.lock().await;
                *current_item = 0;
            }

            let new_videos = VideoFeed::from_config(&config)
                .await
                .expect("Failed to fetch videos")
                .videos;

            if new_videos.len() > 0 {
                let mut videos = videos.lock().await;
                *videos = Some(new_videos);
            }
        })
    }
}

impl Component for Feed {
    fn draw(&mut self, f: &mut Frame, size: Rect) {
        if let Ok(current_item) = self.current_item.try_lock() {
            if let Ok(videos) = self.videos.try_lock() {
                if let Some(ref videos) = *videos {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints(
                            [Constraint::Percentage(80), Constraint::Percentage(20)].as_ref(),
                        )
                        .split(size);

                    let width = f.size().width.into();
                    let list = create_list(&videos, *current_item, width);
                    let description = create_description(&videos, *current_item);

                    f.render_widget(list, chunks[0]);
                    f.render_widget(description, chunks[1]);

                    return;
                }
            }
        }

        self.loading_indicator.draw(f, size);
    }

    fn handle_event(&mut self, event: Event) -> EventFuture {
        if let Event::Key(event) = event {
            match event.code {
                KeyCode::Char(' ') => self.toggle_current_item(),
                KeyCode::Up => self.move_up(),
                KeyCode::Down => self.move_down(),
                KeyCode::Char('j') => self.move_down(),
                KeyCode::Char('k') => self.move_up(),
                KeyCode::Char('r') => self.reload(),
                _ => handled_event(),
            }
        } else {
            handled_event()
        }
    }
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
    let description = videos.get(current_item).unwrap().description.to_owned();

    Paragraph::new(description)
        .block(Block::default().title("Description").borders(Borders::ALL))
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true })
}
