use crate::interface::component::{Backend, Component, EventSender, UpdateEvent};
use crossterm::event::{Event, EventStream};
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
            Some(event) = rx.recv() => {
                match event {
                    UpdateEvent::Redraw => perform_draw(terminal, &mut root),
                    UpdateEvent::Quit => break,
                    UpdateEvent::None => (),
                }
            },
            Some(Ok(event)) = event_reader.next() => handle_event(tx.clone(), &mut root, event),
        };
    }
}

fn handle_event<C>(tx: EventSender, root: &mut C, event: Event)
where
    C: Component,
{
    let future = root.handle_event(event.clone());
    tokio::spawn(async move {
        let event = future.await;
        if event != UpdateEvent::None {
            let _ = tx.send(event).await;
        }
    });
}

fn perform_draw<C>(terminal: &mut Terminal<Backend>, root: &mut C)
where
    C: Component,
{
    terminal
        .draw(|f| root.draw(f, f.size()))
        .expect("Failed to draw");
}
