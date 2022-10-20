use crate::interface::component::{Component, EventSender, Frame};
use tui::layout::Rect;

pub struct Subscriptions {
    tx: EventSender,
}

impl Subscriptions {
    pub fn new(tx: EventSender) -> Self {
        Subscriptions { tx }
    }
}

impl Component for Subscriptions {
    fn draw(&mut self, _f: &mut Frame, _size: Rect) {}
}
