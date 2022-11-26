use super::{
    component::{Component, Frame},
    config_provider::ConfigProvider,
    error_handler::ErrorHandlerActions,
};

use crossterm::event::Event;
use delegate::delegate;
use parking_lot::Mutex;
use std::{fmt::Display, sync::Arc};
use tui::{
    layout::Rect,
    style::{Color, Style},
    widgets::Paragraph,
};

enum LoadingIndicatorMessage {
    Start(usize),
    Stop(usize),
}

#[derive(Clone)]
pub struct LoadingIndicatorActions {
    error_handler_actions: ErrorHandlerActions,
    loading_sender: flume::Sender<LoadingIndicatorMessage>,

    id_counter: Arc<Mutex<usize>>,
}

#[allow(dead_code)]
impl LoadingIndicatorActions {
    pub fn start_loading(&self) -> usize {
        let id = {
            let mut id_counter = self.id_counter.lock();
            *id_counter += 1;
            *id_counter
        };

        self.handle_result(
            self.loading_sender.send(LoadingIndicatorMessage::Start(id)),
            true,
        );

        id
    }

    pub fn finish_loading(&self, id: usize) {
        self.handle_result(
            self.loading_sender.send(LoadingIndicatorMessage::Stop(id)),
            true,
        );
    }

    pub fn loading(&self) -> impl FnOnce() {
        let id = self.start_loading();
        let self_clone = self.clone();
        move || self_clone.finish_loading(id)
    }

    delegate! {
        to self.error_handler_actions {
            pub fn redraw_or_error<T, E: Display>(&self, result: Result<T, E>, ignorable: bool);
            pub async fn redraw_or_error_async<T, E: Display>(&self, result: Result<T, E>, ignorable: bool);
            pub fn handle_result<T, E: Display>(&self, result: Result<T, E>, ignorable: bool);
            pub async fn handle_result_async<T, E: Display>(&self, result: Result<T, E>, ignorable: bool);
            pub fn handle_error<E: Display>(&self, error: E, ignorable: bool);
            pub async fn handle_error_async<E: Display>(&self, error: E, ignorable: bool);
            pub fn redraw(&self);
            pub async fn redraw_async(&self);
            pub fn redraw_fn(&self) -> impl Fn();
        }
    }
}

#[derive(Clone)]
pub struct LoadingIndicator {
    loading_ids: Arc<Mutex<Vec<usize>>>,
    actions: Arc<LoadingIndicatorActions>,
    config_provider: ConfigProvider,
}

impl LoadingIndicator {
    pub fn new(error_handler_actions: ErrorHandlerActions) -> Self {
        let (sender, receiver) = flume::unbounded();
        let actions = LoadingIndicatorActions {
            error_handler_actions,
            loading_sender: sender,
            id_counter: Arc::new(Mutex::new(0)),
        };

        let loading_indicator = Self {
            loading_ids: Default::default(),
            actions: Arc::new(actions.clone()),
            config_provider: ConfigProvider::new(actions),
        };

        loading_indicator.listen_loading_message(receiver);
        loading_indicator
    }

    fn listen_loading_message(&self, loading_receiver: flume::Receiver<LoadingIndicatorMessage>) {
        let actions = self.actions.clone();
        let loading_ids = self.loading_ids.clone();
        tokio::spawn(async move {
            while let Ok(message) = loading_receiver.recv_async().await {
                match message {
                    LoadingIndicatorMessage::Start(id) => {
                        {
                            let mut loading_ids = loading_ids.lock();
                            loading_ids.push(id);
                        }
                        actions.redraw_async().await;
                    }
                    LoadingIndicatorMessage::Stop(id) => {
                        {
                            let mut loading_ids = loading_ids.lock();
                            loading_ids.retain(|value| value != &id);
                        }
                        actions.redraw_async().await;
                    }
                }
            }
        });
    }
}

impl Component for LoadingIndicator {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        if !self.loading_ids.lock().is_empty() {
            let text = "Loading...";
            let label = Paragraph::new(text).style(Style::default().fg(Color::White));
            let text_length = u16::try_from(text.len()).unwrap();
            let label_area = Rect {
                x: area.width - text_length,
                y: 0,
                width: text_length,
                height: 1,
            };
            f.render_widget(label, label_area);
        }

        self.config_provider.draw(f, area);
    }

    fn handle_event(&mut self, event: Event) {
        self.config_provider.handle_event(event);
    }

    fn registered_events(&self) -> Vec<(String, String)> {
        self.config_provider.registered_events()
    }
}
