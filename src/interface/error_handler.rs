use super::{
    component::{Component, Frame},
    dialog::Dialog,
};
use crate::ui::ProgramActions;

use crossterm::event::{Event, KeyCode, KeyEvent};
use parking_lot::Mutex;
use std::{fmt::Display, sync::Arc};
use tui::layout::Rect;

pub struct ErrorMsg {
    pub message: String,
    pub ignorable: bool,
}

#[derive(Clone)]
pub struct ErrorHandlerActions {
    program_actions: ProgramActions,
    error_sender: flume::Sender<ErrorMsg>,
}

#[allow(dead_code)]
impl ErrorHandlerActions {
    pub fn error(&self, error: ErrorMsg) {
        self.error_sender
            .send(error)
            .expect("Failed to send error message");
    }

    pub async fn error_async(&self, error: ErrorMsg) {
        self.error_sender
            .send_async(error)
            .await
            .expect("Failed to send error message");
    }

    pub fn redraw_or_error<T, E: Display>(&self, result: Result<T, E>, ignorable: bool) {
        match result {
            Ok(_) => self.redraw(),
            Err(error) => self.error(ErrorMsg {
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
                self.error_async(ErrorMsg {
                    message: error.to_string(),
                    ignorable,
                })
                .await
            }
        }
    }

    pub fn handle_result<T, E: Display>(&self, result: Result<T, E>) {
        if let Err(error) = result {
            self.error(ErrorMsg {
                message: error.to_string(),
                ignorable: false,
            });
        }
    }

    pub async fn handle_result_async<T, E: Display>(&self, result: Result<T, E>) {
        if let Err(error) = result {
            self.error_async(ErrorMsg {
                message: error.to_string(),
                ignorable: false,
            })
            .await;
        }
    }

    pub fn redraw_fn(&self) -> impl Fn() {
        let clone = self.clone();
        move || clone.redraw()
    }

    // Override program actions by wrapping them in error handling.
    pub fn redraw(&self) {
        self.handle_result(self.program_actions.redraw());
    }

    pub async fn redraw_async(&self) {
        self.handle_result_async(self.program_actions.redraw_async().await)
            .await;
    }
}

pub struct ErrorHandler {
    actions: ErrorHandlerActions,
    child: Box<dyn Component>,
    error: Arc<Mutex<Option<ErrorMsg>>>,
}

impl ErrorHandler {
    pub fn new<C, CF>(program_actions: ProgramActions, child_creator: CF) -> Self
    where
        C: Component + 'static,
        CF: FnOnce(ErrorHandlerActions) -> C,
    {
        let (error_sender, error_receiver) = flume::unbounded();

        let actions = ErrorHandlerActions {
            program_actions,
            error_sender,
        };

        let new_error_handler = Self {
            actions: actions.clone(),
            child: Box::new(child_creator(actions)),
            error: Arc::new(Mutex::new(None)),
        };

        new_error_handler.listen_error_msg(error_receiver);
        new_error_handler
    }

    fn listen_error_msg(&self, error_receiver: flume::Receiver<ErrorMsg>) {
        let actions = self.actions.clone();
        let error = Arc::clone(&self.error);
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
        self.child.draw(f, area);

        if let Some(ref error) = *self.error.lock() {
            Dialog::new_with_body("An error occured", Some(&error.message)).draw(f, area);
        }
    }

    fn handle_event(&mut self, event: Event) {
        let mut error = self.error.lock();
        if let Some(ErrorMsg { ignorable, .. }) = *error {
            if ignorable && event == Event::Key(KeyEvent::from(KeyCode::Esc)) {
                *error = None;
                self.actions.redraw();
            }
        } else {
            self.child.handle_event(event);
        }
    }

    fn registered_events(&self) -> Vec<(String, String)> {
        let error = self.error.lock();
        if let Some(ErrorMsg { ignorable, .. }) = *error {
            if ignorable {
                vec![(String::from("Esc"), String::from("Close"))]
            } else {
                vec![]
            }
        } else {
            self.child.registered_events()
        }
    }
}
