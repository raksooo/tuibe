use crate::interface::component::{Component, Frame};
use tui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

pub struct Dialog {
    text: String,
}

impl Dialog {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
        }
    }

    pub fn update_text(&mut self, text: &str) {
        self.text = text.to_string();
    }
}

impl Component for Dialog {
    fn draw(&mut self, f: &mut Frame, _area: Rect) {
        let area = f.size();
        let dialog = Paragraph::new(self.text.to_string())
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center);

        let area = Rect::new((area.width / 2) - 15, (area.height / 2) - 2, 30, 3);

        f.render_widget(dialog, area);
    }
}
