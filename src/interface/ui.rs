use crate::interface::component::{Backend, Component};
use crate::App;
use crossterm::event::{poll, read};
use crossterm::event::{Event, KeyCode, KeyEvent};
use std::time::Duration;
use tui::Terminal;

pub fn run(terminal: &mut Terminal<Backend>, app: &mut App) {
    loop {
        terminal
            .draw(|f| app.draw(f, f.size()))
            .expect("Failed to draw interface");

        if poll(Duration::from_millis(10)).expect("Failed to poll for input") {
            let event = read().expect("Failed to read input");
            if let Event::Key(KeyEvent {
                code: KeyCode::Char('q'),
                ..
            }) = event
            {
                break;
            } else {
                let future = app.handle_event(event);
                tokio::spawn(future);
            }
        }
    }
}
