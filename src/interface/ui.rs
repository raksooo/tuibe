use crate::interface::component::{Backend, Component};
use crate::App;
use crossterm::event::{Event, KeyCode, KeyEvent, EventStream};
use tokio_stream::StreamExt;
use tokio::{select, sync::mpsc};
use tui::Terminal;

#[derive(Debug)]
enum UpdateEvent {
    Redraw,
    Quit,
}

pub async fn run(terminal: &mut Terminal<Backend>, app: &mut App) {
    let mut event_reader = EventStream::new();
    let (tx, mut rx) = mpsc::channel(100);

    run_draw_cycle(terminal, app);

    loop {
        select! {
            Some(event) = rx.recv() => {
                match event {
                    UpdateEvent::Redraw => run_draw_cycle(terminal, app),
                    UpdateEvent::Quit => break,
                };
            },
            Some(Ok(event)) = event_reader.next() => handle_event(tx.clone(), terminal, app, event),
        };
    }
}

fn handle_event(
    tx: mpsc::Sender<UpdateEvent>,
    terminal: &mut Terminal<Backend>,
    app: &mut App,
    event: Event,
) {
    if let Event::Key(KeyEvent { code: KeyCode::Char('q'), .. }) = event {
        tx.try_send(UpdateEvent::Quit).expect("Failed to send quit event");
    } else {
        app.handle_event_sync(event.clone());
        let future = app.handle_event(event.clone());

        let tx = tx.clone();
        tokio::spawn(async move {
            future.await;
            tx.send(UpdateEvent::Redraw).await.expect("Failed to send draw event");
        });
    }
}

fn run_draw_cycle(terminal: &mut Terminal<Backend>, app: &mut App) {
    terminal
        .draw(|f| app.draw(f, f.size()))
        .expect("Failed to draw interface");
}
