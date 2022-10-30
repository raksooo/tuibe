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
use std::sync::Arc;
use tokio::sync::mpsc;
use tui::layout::{Constraint, Direction, Layout, Rect};

pub enum MainViewMsg {
    CloseConfig,
}

pub struct MainView {
    show_config: Arc<Mutex<bool>>,

    feed: FeedView,
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
        let (main_sender, main_receiver) = mpsc::channel(100);

        let new_main_view = Self {
            show_config: Arc::new(Mutex::new(false)),

            feed: FeedView::new(program_sender.clone(), common_config, videos),
            config: Box::new(config_creator(main_sender)),

            program_sender,
            config_sender,
        };

        new_main_view.listen_main_view_msg(main_receiver);
        new_main_view
    }

    fn listen_main_view_msg(&self, mut main_receiver: mpsc::Receiver<MainViewMsg>) {
        let show_config = Arc::clone(&self.show_config);
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

        self.feed.draw(f, chunks[1]);
    }

    fn handle_event(&mut self, event: Event) {
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
            self.feed.handle_event(event);
        }
    }
}
