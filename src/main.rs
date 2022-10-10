mod error;
mod config;
mod feed;
mod interface;

use config::ConfigHandler;

use std::{thread, time::Duration};
use tui::backend::CrosstermBackend;
use tui::Terminal;
use crossterm::terminal::{
    disable_raw_mode,
    enable_raw_mode,
    EnterAlternateScreen,
    LeaveAlternateScreen,
}

#[tokio::main]
async fn main() {
    let config_handler = ConfigHandler::load().await.expect("Failed to load config");

    enable_raw_mode().expect("Failed to setup interface");
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).expect("Failed to setup interface");

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("Failed to setup interface");

    let feed = interface::feed::Feed::new(config_handler).await;

    terminal
        .draw(|f| {
            let size = f.size();
            let feed_list = feed.render(size);
            f.render_widget(feed_list, size);
        })
        .expect("Failed to draw interface");

    thread::sleep(Duration::from_millis(5000));

    disable_raw_mode().expect("Failed to clean up");
    execute!(terminal.backend_mut(), LeaveAlternateScreen).expect("Failed to clean up");
    terminal.show_cursor().expect("Failed to clean up");

    Ok(())
}
