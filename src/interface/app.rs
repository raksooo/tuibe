use super::{
    component::{Component, Frame, UpdateEvent},
    config_provider::ConfigProvider,
    error_handler::ErrorHandler,
};
use crate::sender_ext::SenderExt;
use crossterm::event::{Event, KeyCode, KeyEvent};
use tokio::sync::mpsc;
use tui::layout::Rect;

pub struct App {
    program_sender: mpsc::Sender<UpdateEvent>,
    error_handler: ErrorHandler,
}

impl App {
    pub fn new(program_sender: mpsc::Sender<UpdateEvent>) -> Self {
        let error_handler = ErrorHandler::new(program_sender.clone(), |error_sender| {
            ConfigProvider::new(program_sender.clone(), error_sender)
        });

        Self {
            program_sender,
            error_handler,
        }
    }
}

impl Component for App {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        self.error_handler.draw(f, area);
    }

    fn handle_event(&mut self, event: Event) {
        if let Event::Key(KeyEvent {
            code: KeyCode::Char('q'),
            ..
        }) = event
        {
            self.program_sender.send_sync(UpdateEvent::Quit);
        } else {
            self.error_handler.handle_event(event);
        }
    }
}
