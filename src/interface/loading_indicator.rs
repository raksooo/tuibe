use crate::interface::component::{Component, Frame, UpdateEvent, UpdateSender};
use futures_timer::Delay;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

pub struct LoadingIndicator {
    dots: Arc<Mutex<usize>>,
    tx: UpdateSender,
}

impl LoadingIndicator {
    pub fn new(tx: UpdateSender) -> Self {
        Self {
            dots: Arc::new(Mutex::new(0)),
            tx,
        }
    }
}

impl Component for LoadingIndicator {
    fn draw(&mut self, f: &mut Frame, size: Rect) {
        if let Ok(dots) = self.dots.try_lock() {
            let dots = format!("{:.<n$}", "", n = dots);
            let dots_with_padding = format!("{:<3}", dots);
            let text = format!("Loading{dots_with_padding}");
            let dialog = Paragraph::new(text)
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Center);

            f.render_widget(dialog, size);

            // TODO: Don't run if draw is called multiple times in a row
            let tx = self.tx.clone();
            let dots = Arc::clone(&self.dots);
            tokio::spawn(async move {
                Delay::new(Duration::from_millis(500)).await;

                {
                    let mut dots = dots.lock().await;
                    *dots += 1;
                    *dots %= 4;
                }

                tx.send(UpdateEvent::Redraw).await;
            });
        }
    }
}
