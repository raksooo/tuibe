use crate::interface::component::{Backend, Component};
use crate::App;
use crossterm::event::{Event, KeyCode, KeyEvent, EventStream};
use tokio_stream::StreamExt;
use tokio::{select, sync::mpsc};
use tui::Terminal;

pub async fn run(terminal: &mut Terminal<Backend>, app: &mut App) {
    let mut event_reader = EventStream::new();
    let (tx, mut rx) = mpsc::channel(100);

    run_draw_cycle(terminal, app);

    loop {
        select! {
            _ = rx.recv() => {
                run_draw_cycle(terminal, app);
            },

            Some(Ok(event)) = event_reader.next() => {
                if let Event::Key(KeyEvent { code: KeyCode::Char('q'), .. }) = event {
                    break;
                } else {
                    app.handle_event_sync(event.clone());
                    let future = app.handle_event(event.clone());

                    let tx = tx.clone();
                    tokio::spawn(async move {
                        future.await;
                        tx.send(()).await.expect("Failed to send draw event");
                    });
                }
            },
        };
    }
}

fn run_draw_cycle(terminal: &mut Terminal<Backend>, app: &mut App) {
    terminal
        .draw(|f| app.draw(f, f.size()))
        .expect("Failed to draw interface");
}
