use crossterm::event::Event;
use std::future;
use tui::{backend::CrosstermBackend, layout::Rect, Frame as TuiFrame};

pub type Backend = CrosstermBackend<std::io::Stdout>;
pub type Frame<'a> = TuiFrame<'a, Backend>;

pub type EventFuture = std::pin::Pin<Box<dyn future::Future<Output = ()> + Send>>;

pub trait Component {
    fn draw(&mut self, f: &mut Frame, size: Rect);

    fn handle_event(&mut self, event: Event) -> EventFuture {
        handled_event()
    }

    fn handle_event_sync(&mut self, _event: Event) {
        ()
    }
}

pub fn handled_event() -> EventFuture {
    Box::pin(future::ready(()))
}
