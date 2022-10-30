use super::{
    component::{Component, Frame, UpdateEvent},
    config_provider::ConfigProvider,
    error_handler::ErrorHandler,
};
use crate::sender_ext::SenderExt;
use crossterm::event::{Event, KeyCode, KeyEvent};
use tokio::sync::mpsc;
use tui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

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

    fn format_events(events: Vec<(String, String)>) -> String {
        events
            .iter()
            .map(|(key, description)| format!("{key}: {description}"))
            .collect::<Vec<String>>()
            .join(" | ")
    }
}

impl Component for App {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        let content_area = Rect::new(area.x, area.y, area.width, area.height - 1);
        let events_area = Rect::new(area.x + 1, area.height - 1, area.width - 2, 1);

        self.error_handler.draw(f, content_area);

        let events = Paragraph::new(Self::format_events(self.registered_events()))
            .block(Block::default().borders(Borders::NONE))
            .style(Style::default().fg(Color::White));

        f.render_widget(events, events_area);
    }

    fn handle_event(&mut self, event: Event) {
        if event == Event::Key(KeyEvent::from(KeyCode::Char('q'))) {
            self.program_sender.send_sync(UpdateEvent::Quit);
        } else {
            self.error_handler.handle_event(event);
        }
    }

    fn registered_events(&self) -> Vec<(String, String)> {
        let mut events = vec![("q".to_string(), "Quit".to_string())];
        events.append(&mut self.error_handler.registered_events());
        events
    }
}
