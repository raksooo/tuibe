use crossterm::event::Event;
use std::{future, pin::Pin};
use tokio::sync::mpsc;
use tui::{backend::CrosstermBackend, layout::Rect, Frame as TuiFrame};

pub type Backend = CrosstermBackend<std::io::Stdout>;
pub type Frame<'a> = TuiFrame<'a, Backend>;

pub type EventFuture = Pin<Box<dyn future::Future<Output = UpdateEvent> + Send>>;

#[derive(Debug, PartialEq)]
pub enum UpdateEvent {
    None,
    Redraw,
    Quit,
}

impl Default for UpdateEvent {
    fn default() -> UpdateEvent {
        UpdateEvent::None
    }
}

pub type EventSender = mpsc::Sender<UpdateEvent>;

pub trait Component {
    fn draw(&mut self, f: &mut Frame, size: Rect);

    fn handle_event(&mut self, _event: Event) -> EventFuture {
        handled_event_none()
    }
}

pub fn handled_event_none() -> EventFuture {
    handled_event(Default::default())
}

pub fn handled_event(event: UpdateEvent) -> EventFuture {
    Box::pin(future::ready(event))
}
