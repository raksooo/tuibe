use crate::interface::component::{Component, EventSender, Frame, UpdateEvent};
use crate::interface::dialog;
use futures_timer::Delay;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tui::layout::Rect;

pub struct LoadingIndicator {
    dots: Arc<Mutex<usize>>,
    tx: EventSender,
}

impl LoadingIndicator {
    pub fn new(tx: EventSender) -> Self {
        Self {
            dots: Arc::new(Mutex::new(0)),
            tx,
        }
    }
}

impl LoadingIndicator {
    fn after_draw(&self) {
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

impl Component for LoadingIndicator {
    fn draw(&mut self, f: &mut Frame, size: Rect) {
        if let Ok(dots) = self.dots.try_lock() {
            let dots = format!("{:.<n$}", "", n = dots);
            let dots_with_padding = format!("{:<3}", dots);
            let text = format!("Loading{dots_with_padding}");

            dialog::dialog(f, size, &text);
            self.after_draw();
        }
    }
}
