use super::{
    actions::Actions,
    backend_provider::BackendProvider,
    component::{Component, Frame},
    error_handler::ErrorHandler,
    status_label::StatusLabel,
    ui::UiMessage,
};

use crossterm::event::{Event, KeyCode};
use ratatui::{
    layout::{Rect, Size},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub struct App {
    actions: Actions,
    error_handler: ErrorHandler<BackendProvider>,
    status_label: StatusLabel,
}

impl App {
    pub fn new(ui_sender: flume::Sender<UiMessage>) -> Self {
        let (error_sender, error_receiver) = flume::unbounded();
        let (status_label_sender, status_label_receiver) = flume::unbounded();

        let actions = Actions::new(ui_sender, error_sender, status_label_sender);
        let config_provider = BackendProvider::new(actions.clone());

        let error_handler = ErrorHandler::new(actions.clone(), error_receiver, config_provider);
        let status_label = StatusLabel::new(actions.clone(), status_label_receiver);

        Self {
            actions,
            error_handler,
            status_label,
        }
    }

    fn format_events(events: Vec<(String, String)>, width: u16) -> Vec<String> {
        let mut lines: Vec<String> = vec![];

        for (key, description) in events.iter() {
            let label = format!("{key}: {description}");
            if let Some(last) = lines.last_mut() {
                if last.len() + label.len() + 3 < width.into() {
                    last.push_str(&format!(" | {}", label));
                    continue;
                }
            }

            lines.push(label);
        }

        lines
    }
}

impl Component for App {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        let events = Self::format_events(self.registered_events(), area.width);
        // It would be unreasonable for the number of command lines to be greater than u16
        let events_height = (events.len() as u16) + 1;

        let status_label_area = Rect::new(0, 0, area.width, 1);
        let content_area = Rect::new(area.x, area.y, area.width, area.height - events_height);
        let events_area = Rect::new(
            area.x + 1,
            area.height - events_height,
            area.width - 2,
            events_height,
        );

        self.error_handler.draw(f, content_area);
        self.status_label.draw(f, status_label_area);

        let events = Paragraph::new(events.join("\n"))
            .block(Block::default().borders(Borders::TOP))
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true });

        f.render_widget(events, events_area);
    }

    fn handle_event(&mut self, event: Event, size: Option<Size>) {
        match event {
            Event::Key(event) if event.code == KeyCode::Char('q') => self.actions.quit(),
            Event::Resize(_, _) => self.actions.redraw(),
            _ => self.error_handler.handle_event(event, size),
        }
    }

    fn registered_events(&self) -> Vec<(String, String)> {
        let mut events = vec![(String::from("q"), String::from("Quit"))];
        events.append(&mut self.error_handler.registered_events());
        events
    }
}
