use crate::interface::component::{Component, Frame, UpdateSender};
use tui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

pub struct LoadingIndicator {
    dots: usize,
}

impl LoadingIndicator {
    pub fn new() -> Self {
        Self { dots: 0 }
    }
}

impl Component for LoadingIndicator {
    fn draw(&mut self, f: &mut Frame, size: Rect) {
        let dots = format!("{:.<n$}", "", n = self.dots);
        let dots_with_padding = format!("{:<3}", dots);
        let text = format!("Loading{dots_with_padding}");
        let dialog = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center);

        f.render_widget(dialog, size);

        self.dots = (self.dots + 1) % 4;
    }
}
