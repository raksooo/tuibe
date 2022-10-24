use crossterm::event::Event;
use tokio::sync::mpsc;
use tui::{backend::CrosstermBackend, layout::Rect, Frame as TuiFrame};

pub type Backend = CrosstermBackend<std::io::Stdout>;
pub type Frame<'a> = TuiFrame<'a, Backend>;

#[derive(Debug)]
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
    fn draw(&mut self, f: &mut Frame, area: Rect);

    fn handle_event(&mut self, _event: Event) -> UpdateEvent {
        UpdateEvent::None
    }
}
