use super::{
    component::{Component, Frame},
    dialog::Dialog,
    loading_indicator::LoadingIndicator,
};
use crate::ui::ProgramActions;

use crossterm::event::{Event, KeyCode, KeyEvent};
use parking_lot::Mutex;
use std::{fmt::Display, sync::Arc};
use tui::layout::Rect;

pub struct ErrorMessage {
    pub message: String,
    pub ignorable: bool,
}

#[derive(Clone)]
pub struct ErrorHandlerActions {
    program_actions: ProgramActions,
    error_sender: flume::Sender<ErrorMessage>,
}

#[allow(dead_code)]
impl ErrorHandlerActions {
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

    pub fn redraw_fn(&self) -> impl Fn() {
        let clone = self.clone();
        move || clone.redraw()
    }

    // Override program actions by wrapping them in error handling.
    pub fn redraw(&self) {
        self.handle_result(self.program_actions.redraw(), false);
    }

    pub async fn redraw_async(&self) {
        self.handle_result_async(self.program_actions.redraw_async().await, false)
            .await;
    }
}

pub struct ErrorHandler {
    actions: ErrorHandlerActions,
    loading_indicator: LoadingIndicator,
    error: Arc<Mutex<Option<ErrorMessage>>>,
}

impl ErrorHandler {
    pub fn new(program_actions: ProgramActions) -> Self {
        let (error_sender, error_receiver) = flume::unbounded();

        let actions = ErrorHandlerActions {
            program_actions,
            error_sender,
        };

        let new_error_handler = Self {
            actions: actions.clone(),
            loading_indicator: LoadingIndicator::new(actions),
            error: Arc::new(Mutex::new(None)),
        };

        new_error_handler.listen_error_msg(error_receiver);
        new_error_handler
    }

    fn listen_error_msg(&self, error_receiver: flume::Receiver<ErrorMessage>) {
        let actions = self.actions.clone();
        let error = self.error.clone();
        tokio::spawn(async move {
            while let Ok(new_error) = error_receiver.recv_async().await {
                {
                    let mut error = error.lock();
                    *error = Some(new_error);
                }
                actions.redraw_async().await;
            }
        });
    }
}

impl Component for ErrorHandler {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        self.loading_indicator.draw(f, area);

        if let Some(ref error) = *self.error.lock() {
            Dialog::new_with_body("An error occured", Some(&error.message)).draw(f, area);
        }
    }

    fn handle_event(&mut self, event: Event) {
        let mut error = self.error.lock();
        if let Some(ErrorMessage { ignorable, .. }) = *error {
            if ignorable && event == Event::Key(KeyEvent::from(KeyCode::Esc)) {
                *error = None;
                self.actions.redraw();
            }
        } else {
            self.loading_indicator.handle_event(event);
        }
    }

    fn registered_events(&self) -> Vec<(String, String)> {
        let error = self.error.lock();
        if let Some(ErrorMessage { ignorable, .. }) = *error {
            if ignorable {
                vec![(String::from("Esc"), String::from("Close"))]
            } else {
                vec![]
            }
        } else {
            self.loading_indicator.registered_events()
        }
    }
}
