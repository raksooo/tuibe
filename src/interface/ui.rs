use crate::App;
use crossterm::event::{poll, read};
use std::time::Duration;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    Frame, Terminal,
};

pub async fn run<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) {
    loop {
        terminal
            .draw(|f| draw(f, &app))
            .expect("Failed to draw interface");

        if !poll_event(app).await {
            break;
        }
    }
}

fn draw<B: Backend>(f: &mut Frame<B>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)].as_ref())
        .split(f.size());

    app.feed.draw(f, chunks[0], chunks[1]);
}

async fn poll_event(app: &mut App) -> bool {
    if poll(Duration::from_millis(10000)).expect("Failed to poll for input") {
        let event = read().expect("Failed to read input");
        if !app.handle_event(event).await {
            false
        } else {
            true
        }
    } else {
        true
    }
}
