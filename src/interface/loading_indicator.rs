use crate::interface::{
    component::{Component, EventSender, Frame, UpdateEvent},
    dialog,
};
use futures_timer::Delay;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
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
                let mut dots = dots.lock().unwrap();
                *dots += 1;
                *dots %= 4;
            }
            let _ = tx.send(UpdateEvent::Redraw).await;
        });
    }
}

impl Component for LoadingIndicator {
    fn draw(&mut self, f: &mut Frame, size: Rect) {
        let dots = self.dots.lock().unwrap();
        let dots_string = format!("{:.<n$}", "", n = dots);
        let dots_with_padding = format!("{:<3}", dots_string);
        let text = format!("Loading{dots_with_padding}");

        dialog::dialog(f, size, &text);

        self.after_draw();
    }
}
