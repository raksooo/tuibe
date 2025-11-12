use crossterm::event::Event;
use ratatui::{backend::CrosstermBackend, layout::Rect, Frame as TuiFrame};

pub type Backend = CrosstermBackend<std::io::Stdout>;
pub type Frame<'a> = TuiFrame<'a>;

pub trait Component {
    fn draw(&mut self, f: &mut Frame, area: Rect);
    fn handle_event(&mut self, _event: Event) {}
    fn registered_events(&self) -> Vec<(String, String)> {
        vec![]
    }
}
