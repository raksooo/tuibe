use super::{
    component::{Component, Frame, UpdateEvent},
    config_provider::ConfigProviderMsg,
    feed_view::FeedView,
};
use crate::{
    config::{common::CommonConfigHandler, config::Video},
    sender_ext::SenderExt,
};
use crossterm::event::{Event, KeyCode};
use parking_lot::Mutex;
use std::{process::Stdio, sync::Arc};
use tokio::{process::Command, sync::mpsc};
use tui::layout::{Constraint, Direction, Layout, Rect};

pub enum AppMsg {
    CloseConfig,
    Play(Vec<Video>),
}

pub struct App {
    show_config: Arc<Mutex<bool>>,

    feed: Arc<Mutex<FeedView>>,
    common_config: Arc<CommonConfigHandler>,
    config: Box<dyn Component + Send>,

    program_sender: mpsc::Sender<UpdateEvent>,
    config_sender: mpsc::Sender<ConfigProviderMsg>,
}

impl App {
    pub fn new<C, CF>(
        program_sender: mpsc::Sender<UpdateEvent>,
        config_sender: mpsc::Sender<ConfigProviderMsg>,
        common_config: CommonConfigHandler,
        videos: Vec<Video>,
        config_creator: CF,
    ) -> Self
    where
        C: Component + Send + 'static,
        CF: FnOnce(mpsc::Sender<AppMsg>) -> C,
    {
        let last_played_timestamp = common_config.config().last_played_timestamp;
        let (app_sender, app_receiver) = mpsc::channel(100);

        let feed = FeedView::new(app_sender.clone(), videos, last_played_timestamp);
        let new_app = Self {
            show_config: Arc::new(Mutex::new(false)),

            feed: Arc::new(Mutex::new(feed)),
            common_config: Arc::new(common_config),
            config: Box::new(config_creator(app_sender)),

            program_sender,
            config_sender,
        };

        new_app.listen_app_msg(app_receiver);

        new_app
    }

    fn listen_app_msg(&self, mut app_receiver: mpsc::Receiver<AppMsg>) {
        let feed = Arc::clone(&self.feed);
        let show_config = Arc::clone(&self.show_config);
        let common_config = Arc::clone(&self.common_config);
        let program_sender = self.program_sender.clone();
        let config_sender = self.config_sender.clone();
        tokio::spawn(async move {
            loop {
                if let Some(msg) = app_receiver.recv().await {
                    match msg {
                        AppMsg::CloseConfig => {
                            let mut show_config = show_config.lock();
                            *show_config = false;
                            config_sender.send_sync(ConfigProviderMsg::Reload);
                        }
                        AppMsg::Play(videos) => {
                            if let Some(newest_video) = videos.get(0) {
                                let new_timestamp = newest_video.date.timestamp();
                                common_config.set_last_played_timestamp(new_timestamp).await;
                                {
                                    let mut feed = feed.lock();
                                    feed.update_last_played_timestamp(new_timestamp);
                                }
                                let _ = program_sender.send(UpdateEvent::Redraw).await;

                                let videos = videos.iter().map(|video| video.url.clone());
                                Command::new(common_config.config().player)
                                    .args(videos)
                                    .stdout(Stdio::null())
                                    .stderr(Stdio::null())
                                    .status()
                                    .await
                                    .unwrap();
                            }
                        }
                    }
                } else {
                    break;
                }
            }
        });
    }

    fn set_show_config(&mut self) -> UpdateEvent {
        let mut show_config = self.show_config.lock();
        *show_config = true;
        UpdateEvent::Redraw
    }
}

impl Component for App {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        let show_config = { self.show_config.lock().to_owned() };
        let config_numerator = if show_config { 1 } else { 0 };

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Ratio(config_numerator, 2),
                    Constraint::Ratio(2 - config_numerator, 2),
                ]
                .as_ref(),
            )
            .split(area);

        if show_config {
            self.config.draw(f, chunks[0]);
        }

        let mut feed = self.feed.lock();
        feed.draw(f, chunks[1]);
    }

    fn handle_event(&mut self, event: Event) -> UpdateEvent {
        if let Event::Key(event) = event {
            match event.code {
                KeyCode::Char('q') => return UpdateEvent::Quit,
                KeyCode::Char('s') => return self.set_show_config(),
                _ => (),
            }
        }

        if *self.show_config.lock() {
            self.config.handle_event(event)
        } else {
            let mut feed = self.feed.lock();
            feed.handle_event(event)
        }
    }
}
