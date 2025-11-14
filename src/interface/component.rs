use crossterm::event::Event;
use ratatui::{
    Frame as TuiFrame,
    backend::CrosstermBackend,
    layout::{Rect, Size},
};

pub type Backend = CrosstermBackend<std::io::Stdout>;
pub type Frame<'a> = TuiFrame<'a>;

pub trait Component {
    fn draw(&mut self, f: &mut Frame, area: Rect);
    fn handle_event(&mut self, _event: Event, _area: Option<Size>) {}
    fn registered_events(&self) -> Vec<(String, String)> {
        vec![]
    }
}
