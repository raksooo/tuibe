use crate::interface::component::{Backend, Component, UpdateEvent, UpdateSender};
use crate::interface::app::App;
use crate::config::ConfigHandler;
use crossterm::event::{Event, EventStream};
use tokio_stream::StreamExt;
use tokio::{select, sync::mpsc};
use tui::Terminal;

pub async fn run(terminal: &mut Terminal<Backend>) {
    let mut event_reader = EventStream::new();
    let (tx, mut rx) = mpsc::channel(100);

    let config_handler = ConfigHandler::load().await.expect("Failed to load config");
    let mut app = App::new(tx.clone(), config_handler);

    {
        run_draw_cycle(terminal, &mut app);
    }

    loop {
        select! {
            Some(event) = rx.recv() => {
                match event {
                    UpdateEvent::Redraw => run_draw_cycle(terminal, &mut app),
                    UpdateEvent::Quit => break,
                };
            },
            Some(Ok(event)) = event_reader.next() => {
                handle_event(tx.clone(), terminal, &mut app, event);
            },
        };
    }
}

fn handle_event(tx: UpdateSender, terminal: &mut Terminal<Backend>, app: &mut App, event: Event) {
    app.handle_event_sync(event.clone());
    let future = app.handle_event(event.clone());

    let tx = tx.clone();
    tokio::spawn(async move {
        future.await;
        tx.send(UpdateEvent::Redraw).await.expect("Failed to send draw event");
    });
}

fn run_draw_cycle(terminal: &mut Terminal<Backend>, app: &mut App) {
    terminal
        .draw(|f| app.draw(f, f.size()))
        .expect("Failed to draw interface");
}
