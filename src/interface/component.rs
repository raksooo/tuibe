use crossterm::event::Event;
use tui::{backend::CrosstermBackend, layout::Rect, Frame as TuiFrame};

pub type Backend = CrosstermBackend<std::io::Stdout>;
pub type Frame<'a> = TuiFrame<'a, Backend>;

pub trait Component {
    fn draw(&self, f: &mut Frame, size: Rect);
    fn handle_event(&mut self, event: Event);
}
