mod config;
mod interface;

use std::{fs::File, str::FromStr};

use config::{rss::RssConfigHandler, Config};
use interface::{app::App, ui};

use crossterm::{
    event::{DisableBracketedPaste, EnableBracketedPaste},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use simplelog::{CombinedLogger, LevelFilter, WriteLogger};
use tui::{backend::CrosstermBackend, Terminal};

#[tokio::main]
async fn main() {
    setup_logging();

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
        import_youtube_takeout(path).await;
    } else {
        run().await;
    }
}

fn setup_logging() {
    let log_level = std::env::var("LOG_LEVEL")
        .ok()
        .unwrap_or_else(|| String::from("Off"));

    CombinedLogger::init(vec![WriteLogger::new(
        LevelFilter::from_str(&log_level).expect("Failed to parse log level"),
        simplelog::Config::default(),
        File::create("tuibe.log").expect("Failed to create log file"),
    )])
    .expect("Failed to set up logger");
}

fn print_help() {
    println!("Available options:");
    println!("  -h|--help                 Show this help message.");
    println!("  --import-youtube <path>   Import subscriptions csv from YouTube takeout");
    println!("  --player <player>         Override player in config");
}

async fn import_youtube_takeout(path: &str) {
    println!("Importing subscriptions...");
    RssConfigHandler::load()
        .await
        .expect("Failed to load config")
        .import_youtube(path)
        .await
        .expect("Failed to import youtube takeout");
    println!("Done.");
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
