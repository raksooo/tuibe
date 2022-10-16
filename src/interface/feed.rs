use crate::{
    config::Config,
    feed::{Feed as VideoFeed, Video},
    interface::{
        component::{handled_event_none, Component, EventFuture, EventSender, Frame, UpdateEvent},
        dialog,
        loading_indicator::LoadingIndicator,
    },
};
use crossterm::event::{Event, KeyCode};
use std::sync::Arc;
use tokio::sync::Mutex;
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
    pub loading_indicator: LoadingIndicator,
}

impl Feed {
    pub fn new(tx: EventSender, config: Option<Config>) -> Self {
        let mut new_feed = Self::create_empty(tx.clone(), config.clone());

        if config.is_some() {
            let reload_future = new_feed.reload();
            tokio::spawn(async move {
                let event = reload_future.await;
                let _ = tx.send(event).await;
            });
        }

        new_feed
    }

    fn create_empty(tx: EventSender, config: Option<Config>) -> Self {
        let videos = Arc::new(Mutex::new(Ok(None)));
        let current_item = Arc::new(Mutex::new(0));

        let loading_indicator = LoadingIndicator::new(tx.clone());

        Self {
            config,
            videos,
            current_item,

            tx,
            loading_indicator,
        }
    }

    pub async fn update_with_config(&mut self, config: &Config) {
        self.config = Some(config.to_owned());
        self.reload().await;
    }

    fn reload(&mut self) -> EventFuture {
        let tx = self.tx.clone();
        let videos = Arc::clone(&self.videos);
        let current_item = Arc::clone(&self.current_item);
        if let Some(config) = &self.config {
            let config = config.clone();
            Box::pin(async move {
                {
                    let mut videos = videos.lock().await;
                    *videos = Ok(None);
                    let mut current_item = current_item.lock().await;
                    *current_item = 0;
                }

                let _ = tx.send(UpdateEvent::Redraw).await;

                let new_videos = VideoFeed::from_config(&config).await;
                let mut videos = videos.lock().await;
                if let Ok(VideoFeed {
                    videos: new_videos, ..
                }) = new_videos
                {
                    if new_videos.len() > 0 {
                        *videos = Ok(Some(new_videos));
                    } else {
                        *videos = Ok(None);
                    }
                } else {
                    *videos = Err(());
                }

                UpdateEvent::Redraw
            })
        } else {
            handled_event_none()
        }
    }

    fn toggle_current_item(&self) -> EventFuture {
        let videos = Arc::clone(&self.videos);
        let current_item = Arc::clone(&self.current_item);
        Box::pin(async move {
            let current_item = current_item.lock().await;
            if let Ok(Some(ref mut videos)) = *videos.lock().await {
                if let Some(video) = videos.get_mut(*current_item) {
                    video.toggle_selected();
                    return UpdateEvent::Redraw;
                }
            }

            UpdateEvent::None
        })
    }

    fn move_up(&mut self) -> EventFuture {
        let current_item = Arc::clone(&self.current_item);
        Box::pin(async move {
            let mut current_item = current_item.lock().await;
            if *current_item > 0 {
                *current_item = *current_item - 1;
                UpdateEvent::Redraw
            } else {
                UpdateEvent::None
            }
        })
    }

    fn move_down(&mut self) -> EventFuture {
        let videos = Arc::clone(&self.videos);
        let current_item = Arc::clone(&self.current_item);
        Box::pin(async move {
            if let Ok(Some(ref videos)) = *videos.lock().await {
                let mut current_item = current_item.lock().await;
                *current_item = std::cmp::min(*current_item + 1, videos.len() - 1);
                UpdateEvent::Redraw
            } else {
                UpdateEvent::None
            }
        })
    }
}

impl Component for Feed {
    fn draw(&mut self, f: &mut Frame, size: Rect) {
        if let Ok(current_item) = self.current_item.try_lock() {
            if let Ok(videos) = self.videos.try_lock() {
                if let Ok(ref videos) = *videos {
                    if let Some(videos) = videos {
                        let description_height = 10;
                        let description_y = size.height - description_height;
                        let list_size = Rect::new(0, 0, size.width, description_y - 10);
                        let description_size =
                            Rect::new(0, description_y, size.width, description_height);

                        let width = f.size().width.into();
                        let list = create_list(&videos, *current_item, width);
                        let description = create_description(&videos, *current_item);

                        f.render_widget(list, list_size);
                        f.render_widget(description, description_size);
                        return;
                    } else {
                        self.loading_indicator.draw(f, size);
                        return;
                    }
                }
            }
        }

        dialog::dialog(f, size, "Something went wrong..");
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
                _ => handled_event_none(),
            }
        } else {
            handled_event_none()
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
    // current_item is always within the bounds of videos
    let description = videos.get(current_item).unwrap().description.to_owned();

    Paragraph::new(description)
        .block(Block::default().title("Description").borders(Borders::ALL))
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true })
}
