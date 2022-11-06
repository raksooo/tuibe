use super::{
    component::{Component, Frame},
    config_provider::ConfigProvider,
    error_handler::ErrorHandler,
};
use crossterm::event::{Event, KeyCode};
use tui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

pub struct App {
    quit_sender: flume::Sender<()>,
    redraw_sender: flume::Sender<()>,
    error_handler: ErrorHandler,
}

impl App {
    pub fn new(quit_sender: flume::Sender<()>, redraw_sender: flume::Sender<()>) -> Self {
        let error_handler = ErrorHandler::new(redraw_sender.clone(), |error_sender| {
            ConfigProvider::new(redraw_sender.clone(), error_sender)
        });

        Self {
            quit_sender,
            redraw_sender,
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
        let events_height = 2;
        let content_area = Rect::new(area.x, area.y, area.width, area.height - events_height);
        let events_area = Rect::new(
            area.x + 1,
            area.height - events_height,
            area.width - 2,
            events_height,
        );

        self.error_handler.draw(f, content_area);

        let events_label = Self::format_events(self.registered_events());
        let events = Paragraph::new(events_label)
            .block(Block::default().borders(Borders::TOP))
            .style(Style::default().fg(Color::White));

        f.render_widget(events, events_area);
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Key(event) if event.code == KeyCode::Char('q') => {
                let _ = self.quit_sender.send(());
            }
            Event::Resize(_, _) => {
                let _ = self.redraw_sender.send(());
            }
            _ => self.error_handler.handle_event(event),
        }
    }

    fn registered_events(&self) -> Vec<(String, String)> {
        let mut events = vec![(String::from("q"), String::from("Quit"))];
        events.append(&mut self.error_handler.registered_events());
        events
    }
}
