use super::component::{Backend, Component, UpdateEvent};
use crossterm::event::EventStream;
use tokio::{
    select,
    sync::{mpsc, mpsc::Sender},
};
use tokio_stream::StreamExt;
use tui::Terminal;

pub async fn create<C, F>(terminal: &mut Terminal<Backend>, creator: F)
where
    C: Component,
    F: FnOnce(Sender<UpdateEvent>) -> C,
{
    let mut event_reader = EventStream::new();
    let (program_sender, mut program_receiver) = mpsc::channel(100);

    let mut root = creator(program_sender.clone());
    program_sender
        .send(UpdateEvent::Redraw)
        .await
        .expect("Failed to send update event");
    loop {
        select! {
            event = event_reader.next() => {
                if let Some(Ok(event)) = event {
                    root.handle_event(event);
                }
            },

            event = program_receiver.recv() => {
                if let Some(event) = event {
                    match event {
                        UpdateEvent::Redraw => perform_draw(terminal, &mut root),
                        UpdateEvent::Quit => break,
                    }
                }
            },
        };
    }
}

fn perform_draw(terminal: &mut Terminal<Backend>, root: &mut impl Component) {
    terminal
        .draw(|f| root.draw(f, f.size()))
        .expect("Failed to draw");
}
