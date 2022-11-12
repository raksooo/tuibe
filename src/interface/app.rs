use super::{
    component::{Component, Frame},
    config_provider::ConfigProvider,
    error_handler::ErrorHandler,
};
use crate::ui::ProgramActions;
use crossterm::event::{Event, KeyCode};
use tui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

pub struct App {
    actions: ProgramActions,
    error_handler: ErrorHandler,
}

impl App {
    pub fn new(actions: ProgramActions) -> Self {
        let error_handler = ErrorHandler::new(actions.clone(), ConfigProvider::new);

        Self {
            actions,
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
                self.actions.quit().expect("Failed to quit");
            }
            Event::Resize(_, _) => {
                let _ = self.actions.redraw();
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
