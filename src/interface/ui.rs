use crate::interface::component::{Backend, Component, EventSender, UpdateEvent};
use crossterm::event::{Event, EventStream};
use futures_timer::Delay;
use std::io;
use std::time::Duration;
use tokio::{select, sync::mpsc};
use tokio_stream::StreamExt;
use tui::Terminal;

pub async fn create<C, F>(terminal: &mut Terminal<Backend>, creator: F)
where
    C: Component,
    F: FnOnce(EventSender) -> C,
{
    let mut event_reader = EventStream::new();
    let (tx, mut rx) = mpsc::channel(100);

    let mut root = creator(tx.clone());
    tx.send(UpdateEvent::Redraw)
        .await
        .expect("Failed to send update event");
    loop {
        select! {
            event = event_reader.next() => handle_input_event(&mut root, tx.clone(), event),
            event = rx.recv() => {
                if let Some(event) = event {
                    match event {
                        UpdateEvent::Redraw => perform_draw(terminal, &mut root),
                        UpdateEvent::Quit => break,
                        UpdateEvent::None => (),
                    }
                }
            },
            // TODO: Remove (helpful if event are broken)
            _ = Delay::new(Duration::from_millis(20000)) => break,
        };
    }
}

fn handle_input_event(
    root: &mut impl Component,
    tx: EventSender,
    event: Option<Result<Event, io::Error>>,
) {
    if let Some(Ok(event)) = event {
        let event = root.handle_event(event);
        tokio::spawn(async move {
            let _ = tx.send(event).await;
        });
    }
}

fn perform_draw(terminal: &mut Terminal<Backend>, root: &mut impl Component) {
    terminal
        .draw(|f| root.draw(f, f.size()))
        .expect("Failed to draw");
}
