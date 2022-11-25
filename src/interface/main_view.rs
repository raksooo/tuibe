use super::{
    component::{Component, Frame},
    config_provider::ConfigProviderActions,
    error_handler::ErrorMessage,
    feed_view::FeedView,
};
use crate::config::{common::CommonConfigHandler, Video};

use crossterm::event::{Event, KeyCode, KeyEvent};
use delegate::delegate;
use parking_lot::Mutex;
use std::{fmt::Display, sync::Arc};
use tui::layout::{Constraint, Direction, Layout, Rect};

pub enum MainViewMessage {
    CloseConfig,
}

#[derive(Clone)]
pub struct MainViewActions {
    config_provider_actions: ConfigProviderActions,
    main_view_sender: flume::Sender<MainViewMessage>,
}

#[allow(dead_code)]
impl MainViewActions {
    pub fn close_config_view(&self) {
        self.config_provider_actions
            .handle_result(self.main_view_sender.send(MainViewMessage::CloseConfig));
    }

    pub async fn close_config_view_async(&self) {
        self.config_provider_actions
            .handle_result_async(
                self.main_view_sender
                    .send_async(MainViewMessage::CloseConfig)
                    .await,
            )
            .await;
    }

    delegate! {
        to self.config_provider_actions {
            pub fn error(&self, error: ErrorMessage);
            pub async fn error_async(&self, error: ErrorMessage);
            pub fn redraw_or_error<T, E: Display>(&self, result: Result<T, E>, ignorable: bool);
            pub async fn redraw_or_error_async<T, E: Display>(&self, result: Result<T, E>, ignorable: bool);
            pub fn handle_result<T, E: Display>(&self, result: Result<T, E>);
            pub async fn handle_result_async<T, E: Display>(&self, result: Result<T, E>);
            pub fn redraw(&self);
            pub async fn redraw_async(&self);
            pub fn redraw_fn(&self) -> impl Fn();
            pub fn reload_config(&self);
            pub async fn reload_config_async(&self);
        }
    }
}

pub struct MainView {
    actions: MainViewActions,

    show_config: Arc<Mutex<bool>>,

    feed: FeedView,
    config: Box<dyn Component + Send>,
}

impl MainView {
    pub fn new<C, CF>(
        config_provider_actions: ConfigProviderActions,
        common_config: CommonConfigHandler,
        videos: Vec<Video>,
        config_creator: CF,
    ) -> Self
    where
        C: Component + Send + 'static,
        CF: FnOnce(MainViewActions) -> C,
    {
        let (main_view_sender, main_view_receiver) = flume::unbounded();
        let actions = MainViewActions {
            config_provider_actions,
            main_view_sender,
        };

        let new_main_view = Self {
            show_config: Arc::new(Mutex::new(false)),

            feed: FeedView::new(actions.clone(), common_config, videos),
            config: Box::new(config_creator(actions.clone())),

            actions,
        };

        new_main_view.listen_main_view_msg(main_view_receiver);
        new_main_view
    }

    fn listen_main_view_msg(&self, main_view_receiver: flume::Receiver<MainViewMessage>) {
        let show_config = self.show_config.clone();
        let actions = self.actions.clone();

        tokio::spawn(async move {
            while let Ok(MainViewMessage::CloseConfig) = main_view_receiver.recv_async().await {
                {
                    let mut show_config = show_config.lock();
                    *show_config = false;
                }
                actions.reload_config_async().await;
            }
        });
    }
}

impl Component for MainView {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        let show_config = self.show_config.lock();
        let config_numerator = u32::from(*show_config);

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
            {
                let mut show_config = self.show_config.lock();
                *show_config = true;
            }
            self.actions.redraw();
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
