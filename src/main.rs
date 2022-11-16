mod config;
mod interface;

use config::{rss::RssConfigHandler, Config};
use interface::{app::App, ui};

use crossterm::{
    event::{DisableBracketedPaste, EnableBracketedPaste},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{backend::CrosstermBackend, Terminal};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print_help();
        return;
    }

    let youtube_export_path = args
        .iter()
        .skip_while(|arg| arg.as_str() != "--import-youtube")
        .nth(1);

    if let Some(path) = youtube_export_path {
        println!("Importing subscriptions...");
        RssConfigHandler::load()
            .await
            .expect("Failed to load config")
            .import_youtube(&path)
            .await
            .expect("Failed to import youtube takeout");
        println!("Done.");
    } else {
        run().await;
    }
}

fn print_help() {
    println!("Available options:");
    println!("  -h|--help                 Show this help message.");
    println!("  --import-youtube <path>   Import subscriptions csv from YouTube takeout");
    println!("  --player <player>         Override player in config");
}

async fn run() {
    enable_raw_mode().expect("Failed to setup interface");
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableBracketedPaste)
        .expect("Failed to setup interface");

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("Failed to setup interface");

    ui::create(&mut terminal, App::new)
        .await
        .expect("Failed to run ui");

    disable_raw_mode().expect("Failed to clean up");
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableBracketedPaste
    )
    .expect("Failed to clean up");
    terminal.show_cursor().expect("Failed to clean up");
}
