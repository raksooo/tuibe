use crate::interface::component::Frame;
use tui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

pub fn dialog(f: &mut Frame, size: Rect, text: &str) {
    let dialog = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center);

    let size = Rect::new((size.width / 2) - 15, (size.height / 2) - 2, 30, 3);

    f.render_widget(dialog, size);
}
