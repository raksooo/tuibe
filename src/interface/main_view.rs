use super::{
    component::{Component, Frame, UpdateEvent},
    config_provider::ConfigProviderMsg,
    dialog::Dialog,
    feed_view::FeedView,
};
use crate::{
    config::{common::CommonConfigHandler, config::Video},
    sender_ext::SenderExt,
};
use crossterm::event::{Event, KeyCode, KeyEvent};
use parking_lot::Mutex;
use std::{process::Stdio, sync::Arc};
use tokio::{process::Command, sync::mpsc};
use tui::layout::{Constraint, Direction, Layout, Rect};

pub enum MainViewMsg {
    CloseConfig,
    Play(Vec<Video>),
}

pub struct MainView {
    show_config: Arc<Mutex<bool>>,
    playing: Arc<Mutex<bool>>,

    feed: Arc<Mutex<FeedView>>,
    common_config: Arc<CommonConfigHandler>,
    config: Box<dyn Component + Send>,

    program_sender: mpsc::Sender<UpdateEvent>,
    config_sender: mpsc::Sender<ConfigProviderMsg>,
}

impl MainView {
    pub fn new<C, CF>(
        program_sender: mpsc::Sender<UpdateEvent>,
        config_sender: mpsc::Sender<ConfigProviderMsg>,
        common_config: CommonConfigHandler,
        videos: Vec<Video>,
        config_creator: CF,
    ) -> Self
    where
        C: Component + Send + 'static,
        CF: FnOnce(mpsc::Sender<MainViewMsg>) -> C,
    {
        let last_played_timestamp = common_config.config().last_played_timestamp;
        let (main_sender, main_receiver) = mpsc::channel(100);

        let feed = FeedView::new(
            program_sender.clone(),
            main_sender.clone(),
            videos,
            last_played_timestamp,
        );
        let new_main_view = Self {
            show_config: Arc::new(Mutex::new(false)),
            playing: Arc::new(Mutex::new(false)),

            feed: Arc::new(Mutex::new(feed)),
            common_config: Arc::new(common_config),
            config: Box::new(config_creator(main_sender)),

            program_sender,
            config_sender,
        };

        new_main_view.listen_main_view_msg(main_receiver);

        new_main_view
    }

    fn listen_main_view_msg(&self, mut main_receiver: mpsc::Receiver<MainViewMsg>) {
        let feed = Arc::clone(&self.feed);
        let show_config = Arc::clone(&self.show_config);
        let playing = Arc::clone(&self.playing);
        let common_config = Arc::clone(&self.common_config);
        let program_sender = self.program_sender.clone();
        let config_sender = self.config_sender.clone();
        tokio::spawn(async move {
            loop {
                if let Some(msg) = main_receiver.recv().await {
                    match msg {
                        MainViewMsg::CloseConfig => {
                            let mut show_config = show_config.lock();
                            *show_config = false;
                            config_sender.send_sync(ConfigProviderMsg::Reload);
                        }
                        MainViewMsg::Play(videos) => {
                            if let Some(newest_video) = videos.get(0) {
                                let new_timestamp = newest_video.date.timestamp();
                                common_config.set_last_played_timestamp(new_timestamp).await;
                                {
                                    let mut feed = feed.lock();
                                    feed.update_last_played_timestamp(new_timestamp);
                                    let mut playing = playing.lock();
                                    *playing = true;
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

                                {
                                    let mut playing = playing.lock();
                                    *playing = false;
                                }
                                let _ = program_sender.send(UpdateEvent::Redraw).await;
                            }
                        }
                    }
                } else {
                    break;
                }
            }
        });
    }

    fn set_show_config(&mut self) {
        let mut show_config = self.show_config.lock();
        *show_config = true;
        self.program_sender.send_sync(UpdateEvent::Redraw);
    }
}

impl Component for MainView {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        let show_config = self.show_config.lock();
        let config_numerator = if *show_config { 1 } else { 0 };

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

        if *show_config {
            self.config.draw(f, chunks[0]);
        }

        let playing = self.playing.lock();
        if *playing {
            Dialog::new("Playing selection.").draw(f, area);
        }

        let mut feed = self.feed.lock();
        feed.draw(f, chunks[1]);
    }

    fn handle_event(&mut self, event: Event) {
        {
            let mut playing = self.playing.lock();
            if *playing {
                if let Event::Key(KeyEvent {
                    code: KeyCode::Esc, ..
                }) = event
                {
                    *playing = false;
                    self.program_sender.send_sync(UpdateEvent::Redraw);
                }
                return;
            }
        }

        if let Event::Key(event) = event {
            match event.code {
                KeyCode::Char('q') => return self.program_sender.send_sync(UpdateEvent::Quit),
                KeyCode::Char('c') => return self.set_show_config(),
                _ => (),
            }
        }

        if *self.show_config.lock() {
            self.config.handle_event(event);
        } else {
            let mut feed = self.feed.lock();
            feed.handle_event(event);
        }
    }
}
