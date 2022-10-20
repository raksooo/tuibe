use crate::interface::{
    component::{Component, EventSender, Frame, UpdateEvent},
    dialog,
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
    handle: JoinHandle<()>,
}

impl LoadingIndicator {
    pub fn new(tx: EventSender) -> Self {
        let dots = Arc::new(Mutex::new(0));
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

        Self { dots, handle }
    }
}

impl Drop for LoadingIndicator {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

impl Component for LoadingIndicator {
    fn draw(&mut self, f: &mut Frame, size: Rect) {
        let dots = self.dots.lock().unwrap();
        let dots_string = format!("{:.<n$}", "", n = dots);
        let dots_with_padding = format!("{:<3}", dots_string);
        let text = format!("Loading{dots_with_padding}");

        dialog::dialog(f, size, &text);
    }
}
