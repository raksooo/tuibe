use crate::interface::component::{Component, Frame};
use tui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

pub struct Dialog {
    title: String,
    body: Option<String>,
}

impl Dialog {
    pub fn new(title: &str, body: Option<&str>) -> Self {
        Self {
            title: title.to_string(),
            body: body.map(|value| value.to_owned()),
        }
    }

    pub fn update_text(&mut self, title: &str, body: Option<&str>) {
        self.title = title.to_string();
        self.body = body.map(|value| value.to_owned());
    }
}

impl Component for Dialog {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        if let Some(body) = &self.body {
            let block = Block::default()
                .title(self.title.clone())
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White))
                .style(Style::default().bg(Color::Black));

            let dialog = Paragraph::new(body.clone())
                .block(block)
                .style(Style::default().fg(Color::White))
                .wrap(Wrap { trim: true });

            let area = Rect::new((area.width / 2) - 20, (area.height / 2) - 3, 40, 6);

            f.render_widget(Clear, area);
            f.render_widget(dialog, area);
        } else {
            let dialog = Paragraph::new(self.title.to_string())
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Center);

            let area = Rect::new((area.width / 2) - 15, (area.height / 2) - 2, 30, 3);

            f.render_widget(Clear, area);
            f.render_widget(dialog, area);
        }
    }
}
