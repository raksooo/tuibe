use super::{
    component::{Component, Frame, UpdateEvent},
    dialog::Dialog,
};
use crate::sender_ext::SenderExt;
use async_trait::async_trait;
use crossterm::event::{Event, KeyCode, KeyEvent};
use parking_lot::Mutex;
use std::{fmt::Display, future::Future, sync::Arc};
use tokio::sync::mpsc;
use tui::layout::Rect;

pub struct ErrorMsg {
    pub message: String,
    pub ignorable: bool,
}

pub struct ErrorHandler {
    program_sender: mpsc::Sender<UpdateEvent>,
    child: Box<dyn Component>,
    error: Arc<Mutex<Option<ErrorMsg>>>,
}

impl ErrorHandler {
    pub fn new<C, CF>(program_sender: mpsc::Sender<UpdateEvent>, child_creator: CF) -> Self
    where
        C: Component + 'static,
        CF: FnOnce(mpsc::Sender<ErrorMsg>) -> C,
    {
        let (error_sender, error_receiver) = mpsc::channel(10);

        let new_error_handler = Self {
            program_sender,
            child: Box::new(child_creator(error_sender)),
            error: Arc::new(Mutex::new(None)),
        };

        new_error_handler.listen_error_msg(error_receiver);
        new_error_handler
    }

    fn listen_error_msg(&self, mut error_receiver: mpsc::Receiver<ErrorMsg>) {
        let program_sender = self.program_sender.clone();
        let error = Arc::clone(&self.error);
        tokio::spawn(async move {
            while let Some(new_error) = error_receiver.recv().await {
                {
                    let mut error = error.lock();
                    *error = Some(new_error);
                }
                program_sender.send_sync(UpdateEvent::Redraw);
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
                self.program_sender.send_sync(UpdateEvent::Redraw);
            }
        } else {
            self.child.handle_event(event);
        }
    }

    fn registered_events(&self) -> Vec<(String, String)> {
        let error = self.error.lock();
        if let Some(ErrorMsg { ignorable, .. }) = *error {
            if ignorable {
                vec![("Esc".to_string(), "Close".to_string())]
            } else {
                vec![]
            }
        } else {
            self.child.registered_events()
        }
    }
}

#[async_trait]
pub trait ErrorSenderExt {
    fn run_or_send<T, E, F>(&self, result: Result<T, E>, ignorable: bool, f: F)
    where
        T: Send,
        E: Display,
        F: FnOnce(T);

    async fn run_or_send_async<T, E, F, R>(&self, result: Result<T, E>, ignorable: bool, f: F)
    where
        T: Send,
        E: Display + Send + Sync,
        R: Future<Output = ()> + Send,
        F: FnOnce(T) -> R + Send;
}

#[async_trait]
impl ErrorSenderExt for mpsc::Sender<ErrorMsg> {
    fn run_or_send<T, E, F>(&self, result: Result<T, E>, ignorable: bool, f: F)
    where
        T: Send,
        E: Display,
        F: FnOnce(T),
    {
        match result {
            Ok(value) => f(value),
            Err(err) => self.send_sync(ErrorMsg {
                message: err.to_string(),
                ignorable,
            }),
        }
    }

    async fn run_or_send_async<T, E, F, R>(&self, result: Result<T, E>, ignorable: bool, f: F)
    where
        T: Send,
        E: Display + Send + Sync,
        R: Future<Output = ()> + Send,
        F: FnOnce(T) -> R + Send,
    {
        match result {
            Ok(value) => f(value).await,
            Err(err) => {
                let _ = self
                    .send(ErrorMsg {
                        message: err.to_string(),
                        ignorable,
                    })
                    .await;
            }
        }
    }
}
