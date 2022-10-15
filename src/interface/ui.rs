use crate::interface::app::App;
use crate::interface::component::{Backend, Component, EventSender, UpdateEvent};
use crossterm::event::{Event, EventStream};
use tokio::{select, sync::mpsc};
use tokio_stream::StreamExt;
use tui::Terminal;

pub async fn run(terminal: &mut Terminal<Backend>) {
    let mut event_reader = EventStream::new();
    let (tx, mut rx) = mpsc::channel(100);

    let mut app: Box<dyn Component> = Box::new(App::new(tx.clone()));

    tx.send(UpdateEvent::Redraw).await;
    loop {
        select! {
            Some(event) = rx.recv() => {
                match event {
                    UpdateEvent::Redraw => run_draw_cycle(terminal, &mut app),
                    UpdateEvent::Quit => break,
                    UpdateEvent::None => (),
                };
            },
            Some(Ok(event)) = event_reader.next() => {
                handle_event(tx.clone(), &mut app, event);
            },
        };
    }
}

fn handle_event(tx: EventSender, app: &mut Box<dyn Component>, event: Event) {
    let future = app.handle_event(event.clone());
    tokio::spawn(async move {
        let event = future.await;
        if event != UpdateEvent::None {
            tx.send(event).await.expect("Failed to send draw event");
        }
    });
}

fn run_draw_cycle(terminal: &mut Terminal<Backend>, app: &mut Box<dyn Component>) {
    terminal
        .draw(|f| app.draw(f, f.size()))
        .expect("Failed to draw interface");
}
