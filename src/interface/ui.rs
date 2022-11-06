use super::component::{Backend, Component, UpdateEvent};
use crossterm::event::EventStream;
use tokio::select;
use tokio_stream::StreamExt;
use tui::Terminal;

pub async fn create<C, F>(terminal: &mut Terminal<Backend>, creator: F)
where
    C: Component,
    F: FnOnce(flume::Sender<UpdateEvent>) -> C,
{
    let mut event_reader = EventStream::new();
    let (program_sender, program_receiver) = flume::unbounded();

    let mut root = creator(program_sender.clone());
    program_sender
        .send_async(UpdateEvent::Redraw)
        .await
        .expect("Failed to send update event");
    loop {
        select! {
            event = event_reader.next() => {
                if let Some(Ok(event)) = event {
                    root.handle_event(event);
                }
            },

            event = program_receiver.recv_async() => {
                if let Ok(event) = event {
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
