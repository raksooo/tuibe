use crossterm::event::Event;
use tokio::sync::mpsc;
use tui::{backend::CrosstermBackend, layout::Rect, Frame as TuiFrame};

pub type Backend = CrosstermBackend<std::io::Stdout>;
pub type Frame<'a> = TuiFrame<'a, Backend>;

#[derive(Debug)]
pub enum UpdateEvent {
    Redraw,
    Quit,
}

pub type EventSender = mpsc::Sender<UpdateEvent>;

pub trait Component {
    fn draw(&mut self, f: &mut Frame, area: Rect);
    fn handle_event(&mut self, _event: Event) {}
    fn registered_events(&self) -> Vec<(String, String)> {
        vec![]
    }
}
