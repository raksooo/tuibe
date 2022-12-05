use super::{
    actions::Actions,
    component::{Component, Frame},
};

use parking_lot::Mutex;
use std::{collections::HashMap, sync::Arc};
use tui::{
    layout::Rect,
    style::{Color, Style},
    widgets::Paragraph,
};

pub const LOADING_STRING: &str = "Loading...";

pub enum StatusLabelMessage {
    Show(usize, String),
    Remove(usize),
}

#[derive(Clone)]
pub struct StatusLabel {
    status_labels: Arc<Mutex<HashMap<usize, String>>>,
    actions: Actions,
}

impl StatusLabel {
    pub fn new(actions: Actions, receiver: flume::Receiver<StatusLabelMessage>) -> Self {
        let status_label = Self {
            status_labels: Default::default(),
            actions,
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
    }
}
