use super::{error_handler::ErrorMessage, status_label::StatusLabelMessage, ui::UiMessage};

use parking_lot::Mutex;
use std::{fmt::Display, sync::Arc};

#[derive(Clone)]
pub struct Actions {
    ui_sender: flume::Sender<UiMessage>,

    error_sender: flume::Sender<ErrorMessage>,

    status_label_sender: flume::Sender<StatusLabelMessage>,
    status_label_id_counter: Arc<Mutex<usize>>,
}

impl Actions {
    pub fn new(
        ui_sender: flume::Sender<UiMessage>,
        error_sender: flume::Sender<ErrorMessage>,
        status_label_sender: flume::Sender<StatusLabelMessage>,
    ) -> Self {
        Self {
            ui_sender,

            error_sender,

            status_label_sender,
            status_label_id_counter: Arc::new(Mutex::new(0)),
        }
    }
}

// Implement UI actions
#[allow(dead_code)]
impl Actions {
    pub fn quit(&self) {
        self.handle_result(self.ui_sender.send(UiMessage::Quit), false);
    }

    pub async fn quit_async(&self) {
        self.handle_result_async(self.ui_sender.send_async(UiMessage::Quit).await, false)
            .await;
    }

    pub fn redraw(&self) {
        self.handle_result(self.ui_sender.send(UiMessage::Redraw), false);
    }

    pub async fn redraw_async(&self) {
        self.handle_result_async(self.ui_sender.send_async(UiMessage::Redraw).await, false)
            .await;
    }
}

// Implement error actions
#[allow(dead_code)]
impl Actions {
    fn error(&self, error: ErrorMessage) {
        self.error_sender
            .send(error)
            .expect("Failed to send error message");
    }

    async fn error_async(&self, error: ErrorMessage) {
        self.error_sender
            .send_async(error)
            .await
            .expect("Failed to send error message");
    }

    pub fn redraw_or_error<T, E: Display>(&self, result: Result<T, E>, ignorable: bool) {
        match result {
            Ok(_) => self.redraw(),
            Err(error) => self.error(ErrorMessage {
                message: error.to_string(),
                ignorable,
            }),
        }
    }

    pub async fn redraw_or_error_async<T, E: Display>(
        &self,
        result: Result<T, E>,
        ignorable: bool,
    ) {
        match result {
            Ok(_) => self.redraw_async().await,
            Err(error) => {
                self.error_async(ErrorMessage {
                    message: error.to_string(),
                    ignorable,
                })
                .await
            }
        }
    }

    pub fn handle_error<E: Display>(&self, error: E, ignorable: bool) {
        self.error(ErrorMessage {
            message: error.to_string(),
            ignorable,
        });
    }

    pub async fn handle_error_async<E: Display>(&self, error: E, ignorable: bool) {
        self.error_async(ErrorMessage {
            message: error.to_string(),
            ignorable,
        })
        .await;
    }

    pub fn handle_result<T, E: Display>(&self, result: Result<T, E>, ignorable: bool) {
        if let Err(error) = result {
            self.handle_error(error, ignorable);
        }
    }

    pub async fn handle_result_async<T, E: Display>(&self, result: Result<T, E>, ignorable: bool) {
        if let Err(error) = result {
            self.handle_error_async(error, ignorable).await;
        }
    }
}

// Implement status label actions
#[allow(dead_code)]
impl Actions {
    pub fn start_status(&self, label: &str) -> usize {
        let id = {
            let mut id_counter = self.status_label_id_counter.lock();
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

    pub fn show_label(&self, label: &str) -> impl FnOnce() + use<> {
        let id = self.start_status(label);
        let self_clone = self.clone();
        move || self_clone.finish_status(id)
    }
}
