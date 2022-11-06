use super::component::{Backend, Component};
use crossterm::event::EventStream;
use tokio::select;
use tokio_stream::StreamExt;
use tui::Terminal;

pub async fn create<C, F>(terminal: &mut Terminal<Backend>, creator: F)
where
    C: Component,
    F: FnOnce(flume::Sender<()>, flume::Sender<()>) -> C,
{
    let mut event_reader = EventStream::new();
    let (quit_sender, quit_receiver) = flume::unbounded();
    let (redraw_sender, redraw_receiver) = flume::unbounded();

    let mut root = creator(quit_sender, redraw_sender.clone());
    redraw_sender
        .send_async(())
        .await
        .expect("Failed to send update event");
    loop {
        select! {
            event = event_reader.next() => {
                if let Some(Ok(event)) = event {
                    root.handle_event(event);
                }
            },

            _ = quit_receiver.recv_async() => break,

            event = redraw_receiver.recv_async() => {
                if event.is_ok() {
                    redraw_receiver.drain();
                    perform_draw(terminal, &mut root);
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
