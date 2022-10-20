use crate::interface::{
    component::{Component, EventSender, Frame, UpdateEvent},
    dialog::Dialog,
};
use futures_timer::Delay;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::task::JoinHandle;
use tui::layout::Rect;

pub struct LoadingIndicator {
    dots: Arc<Mutex<usize>>,
    dialog: Dialog,
    handle: JoinHandle<()>,
}

impl LoadingIndicator {
    pub fn new(tx: EventSender) -> Self {
        let dots = Arc::new(Mutex::new(0));
        let dialog = Dialog::new(&Self::format_text(0));
        let dots_async = Arc::clone(&dots);
        let handle = tokio::spawn(async move {
            Delay::new(Duration::from_millis(500)).await;
            {
                let mut dots = dots_async.lock().unwrap();
                *dots += 1;
                *dots %= 4;
            }
            let _ = tx.send(UpdateEvent::Redraw).await;
        });

        Self {
            dots,
            dialog,
            handle,
        }
    }

    fn before_draw(&mut self) {
        let dots = self.dots.lock().unwrap();
        let text = Self::format_text(*dots);
        self.dialog.update_text(&text);
    }

    fn format_text(dots: usize) -> String {
        let dots_string = format!("{:.<n$}", "", n = dots);
        let dots_with_padding = format!("{:<3}", dots_string);
        format!("Loading{dots_with_padding}")
    }
}

impl Drop for LoadingIndicator {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

impl Component for LoadingIndicator {
    fn draw(&mut self, f: &mut Frame, size: Rect) {
        self.before_draw();
        self.dialog.draw(f, size);
    }
}
