mod config;
mod error;
mod feed;
mod interface;

use config::ConfigHandler;
use interface::app::App;
use interface::ui;

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{backend::CrosstermBackend, Terminal};

#[tokio::main]
async fn main() {
    enable_raw_mode().expect("Failed to setup interface");
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen).expect("Failed to setup interface");

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("Failed to setup interface");

    let config_handler = ConfigHandler::load().await.expect("Failed to load config");
    let mut app = interface::app::App::new(config_handler);

    ui::run(&mut terminal, &mut app).await;

    disable_raw_mode().expect("Failed to clean up");
    execute!(terminal.backend_mut(), LeaveAlternateScreen).expect("Failed to clean up");
    terminal.show_cursor().expect("Failed to clean up");
}
