use crossterm::event::Event;
use std::future;
use tokio::sync::mpsc;
use tui::{backend::CrosstermBackend, layout::Rect, Frame as TuiFrame};

pub type Backend = CrosstermBackend<std::io::Stdout>;
pub type Frame<'a> = TuiFrame<'a, Backend>;

pub type EventFuture = std::pin::Pin<Box<dyn future::Future<Output = ()> + Send>>;

#[derive(Debug)]
pub enum UpdateEvent {
    Redraw,
    Quit,
}

pub type UpdateSender = mpsc::Sender<UpdateEvent>;

pub trait Component<P> {
    fn new(tx: UpdateSender, props: P) -> Self
    where
        Self: Sized;

    fn draw(&mut self, f: &mut Frame, size: Rect);

    fn handle_event(&mut self, _event: Event) -> EventFuture {
        handled_event()
    }

    fn handle_event_sync(&mut self, _event: Event) {
        ()
    }
}

pub fn handled_event() -> EventFuture {
    Box::pin(future::ready(()))
}
