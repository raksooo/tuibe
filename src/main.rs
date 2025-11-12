mod backend;
mod config;
mod config_error;
mod file_handler;
mod interface;

use std::{
    fs::{self, File},
    path::PathBuf,
    str::FromStr,
};

use backend::{rss::RssBackend, Backend};
use interface::{app::App, ui};

use crossterm::{
    event::{DisableBracketedPaste, EnableBracketedPaste},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use simplelog::{CombinedLogger, LevelFilter, WriteLogger};

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

fn print_help() {
    println!("Available options:");
    println!("  -h|--help                 Show this help message.");
    println!("  --import-youtube <path>   Import subscriptions csv from YouTube takeout");
    println!("  --player <player>         Override player in config");
}

async fn import_youtube_takeout(path: &str) {
    println!("Importing subscriptions...");
    RssBackend::load()
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

fn setup_logging() {
    let log_level = std::env::var("LOG_LEVEL")
        .ok()
        .unwrap_or_else(|| String::from("Off"));

    let config_path = get_config_file_path();
    fs::create_dir_all(config_path.parent().expect("Invalid path"))
        .expect("Failed to create log dir");
    CombinedLogger::init(vec![WriteLogger::new(
        LevelFilter::from_str(&log_level).expect("Failed to parse log level"),
        simplelog::Config::default(),
        File::create(config_path).expect("Failed to create log file"),
    )])
    .expect("Failed to set up logger");
}

pub fn get_config_file_path() -> PathBuf {
    let mut path = PathBuf::new();

    match std::env::var("XDG_STATE_HOME") {
        Ok(config_dir) => path.push(config_dir),
        _ => {
            let home = std::env::var("HOME").unwrap_or(String::from("."));
            path.push(home);
            path.push(".local/state");
        }
    }

    path.push("tuibe/tuibe.log");
    path
}
