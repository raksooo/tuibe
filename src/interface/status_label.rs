use super::{
    component::{Component, Frame},
    config_provider::ConfigProvider,
    error_handler::ErrorHandlerActions,
};

use crossterm::event::Event;
use delegate::delegate;
use parking_lot::Mutex;
use std::{collections::HashMap, fmt::Display, sync::Arc};
use tui::{
    layout::Rect,
    style::{Color, Style},
    widgets::Paragraph,
};

pub const LOADING_STRING: &str = "Loading...";

enum StatusLabelMessage {
    Show(usize, String),
    Remove(usize),
}

#[derive(Clone)]
pub struct StatusLabelActions {
    error_handler_actions: ErrorHandlerActions,
    status_label_sender: flume::Sender<StatusLabelMessage>,

    id_counter: Arc<Mutex<usize>>,
}

#[allow(dead_code)]
impl StatusLabelActions {
    pub fn start_status(&self, label: &str) -> usize {
        let id = {
            let mut id_counter = self.id_counter.lock();
            *id_counter += 1;
            *id_counter
        };

        self.handle_result(
            self.status_label_sender
                .send(StatusLabelMessage::Show(id, label.to_owned())),
            true,
        );

        id
    }

    pub fn finish_status(&self, id: usize) {
        self.handle_result(
            self.status_label_sender
                .send(StatusLabelMessage::Remove(id)),
            true,
        );
    }

    pub fn show_label(&self, label: &str) -> impl FnOnce() {
        let id = self.start_status(label);
        let self_clone = self.clone();
        move || self_clone.finish_status(id)
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
pub struct StatusLabel {
    status_labels: Arc<Mutex<HashMap<usize, String>>>,
    actions: Arc<StatusLabelActions>,
    config_provider: ConfigProvider,
}

impl StatusLabel {
    pub fn new(error_handler_actions: ErrorHandlerActions) -> Self {
        let (sender, receiver) = flume::unbounded();
        let actions = StatusLabelActions {
            error_handler_actions,
            status_label_sender: sender,
            id_counter: Arc::new(Mutex::new(0)),
        };

        let status_label = Self {
            status_labels: Default::default(),
            actions: Arc::new(actions.clone()),
            config_provider: ConfigProvider::new(actions),
        };

        status_label.listen_status_label_messages(receiver);
        status_label
    }

    fn listen_status_label_messages(&self, receiver: flume::Receiver<StatusLabelMessage>) {
        let actions = self.actions.clone();
        let status_labels = self.status_labels.clone();
        tokio::spawn(async move {
            while let Ok(message) = receiver.recv_async().await {
                match message {
                    StatusLabelMessage::Show(id, label) => {
                        {
                            status_labels.lock().insert(id, label);
                        }
                        actions.redraw_async().await;
                    }
                    StatusLabelMessage::Remove(id) => {
                        {
                            status_labels.lock().remove(&id);
                        }
                        actions.redraw_async().await;
                    }
                }
            }
        });
    }
}

impl Component for StatusLabel {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        let status_labels = self.status_labels.lock();
        if !status_labels.is_empty() {
            let labels: Vec<&str> = status_labels.values().map(|label| label.as_ref()).collect();
            let text: &str = &labels.join(", ");
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
