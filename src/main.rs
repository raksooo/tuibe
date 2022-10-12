mod error;
mod config;
mod feed;
mod interface;

use config::ConfigHandler;
use interface::app::App;

use std::time::Duration;
use tui::{backend::{Backend, CrosstermBackend}, Terminal, Frame};
use crossterm::{
    event::{poll, read},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

#[tokio::main]
async fn main() {
    enable_raw_mode().expect("Failed to setup interface");
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen).expect("Failed to setup interface");

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("Failed to setup interface");

    let config_handler = ConfigHandler::load().await.expect("Failed to load config");
    let mut app = interface::app::App::new(config_handler).await;

    run_event_loop(&mut terminal, &mut app);

    disable_raw_mode().expect("Failed to clean up");
    execute!(terminal.backend_mut(), LeaveAlternateScreen).expect("Failed to clean up");
    terminal.show_cursor().expect("Failed to clean up");
}

fn run_event_loop<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) {
    loop {
        terminal.draw(|f| draw(f, &app)).expect("Failed to draw interface");
        if !poll_event(app) {
            break;
        }
    }
}

fn draw<B: Backend>(f: &mut Frame<B>, app: &App) {
    let size = f.size();
    f.render_widget(app.feed.render(size), size);
}

fn poll_event(app: &mut App) -> bool {
    if poll(Duration::from_millis(10000)).expect("Failed to poll for input") {
        let event = read().expect("Failed to read input");
        if !app.handle_event(event) {
            false
        } else {
            true
        }
    } else {
        true
    }
}
