use super::{
    component::{Component, Frame, UpdateEvent},
    config_provider::ConfigProviderMsg,
    error_handler::ErrorMsg,
    feed_view::FeedView,
};
use crate::config::{common::CommonConfigHandler, config::Video};
use crossterm::event::{Event, KeyCode, KeyEvent};
use parking_lot::Mutex;
use std::sync::Arc;
use tui::layout::{Constraint, Direction, Layout, Rect};

pub enum MainViewMsg {
    CloseConfig,
}

pub struct MainView {
    show_config: Arc<Mutex<bool>>,

    feed: FeedView,
    config: Box<dyn Component + Send>,

    program_sender: flume::Sender<UpdateEvent>,
    config_sender: flume::Sender<ConfigProviderMsg>,
}

impl MainView {
    pub fn new<C, CF>(
        program_sender: flume::Sender<UpdateEvent>,
        error_sender: flume::Sender<ErrorMsg>,
        config_sender: flume::Sender<ConfigProviderMsg>,
        common_config: CommonConfigHandler,
        videos: Vec<Video>,
        config_creator: CF,
    ) -> Self
    where
        C: Component + Send + 'static,
        CF: FnOnce(flume::Sender<MainViewMsg>) -> C,
    {
        let (main_sender, main_receiver) = flume::unbounded();

        let new_main_view = Self {
            show_config: Arc::new(Mutex::new(false)),

            feed: FeedView::new(program_sender.clone(), error_sender, common_config, videos),
            config: Box::new(config_creator(main_sender)),

            program_sender,
            config_sender,
        };

        new_main_view.listen_main_view_msg(main_receiver);
        new_main_view
    }

    fn listen_main_view_msg(&self, main_receiver: flume::Receiver<MainViewMsg>) {
        let show_config = Arc::clone(&self.show_config);
        let config_sender = self.config_sender.clone();

        tokio::spawn(async move {
            while let Ok(MainViewMsg::CloseConfig) = main_receiver.recv_async().await {
                let mut show_config = show_config.lock();
                *show_config = false;
                let _ = config_sender.send(ConfigProviderMsg::Reload);
            }
        });
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
        if *self.show_config.lock() {
            self.config.handle_event(event);
        } else if event == Event::Key(KeyEvent::from(KeyCode::Char('c'))) {
            let mut show_config = self.show_config.lock();
            *show_config = true;
            let _ = self.program_sender.send(UpdateEvent::Redraw);
        } else {
            self.feed.handle_event(event);
        }
    }

    fn registered_events(&self) -> Vec<(String, String)> {
        if *self.show_config.lock() {
            self.config.registered_events()
        } else {
            let mut events = vec![(String::from("c"), String::from("Configure"))];
            events.append(&mut self.feed.registered_events());
            events
        }
    }
}
