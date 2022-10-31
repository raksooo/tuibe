mod sender_ext;

mod config;
mod interface;

use interface::{app::App, ui};
use config::{config::Config, rss::RssConfigHandler};

use crossterm::{
    event::{DisableBracketedPaste, EnableBracketedPaste},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{backend::CrosstermBackend, Terminal};

#[tokio::main]
async fn main() {
    if let Some(path) = std::env::args().skip_while(|arg| arg != "--import-youtube").nth(1) {
        RssConfigHandler::load()
            .await
            .expect("Failed to load config")
            .import_youtube(path)
            .await;
    } else {
        run().await;
    }
}

async fn run() {
    enable_raw_mode().expect("Failed to setup interface");
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableBracketedPaste)
        .expect("Failed to setup interface");

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("Failed to setup interface");

    ui::create(&mut terminal, |program_sender| App::new(program_sender)).await;

    disable_raw_mode().expect("Failed to clean up");
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableBracketedPaste
    )
    .expect("Failed to clean up");
    terminal.show_cursor().expect("Failed to clean up");
}
