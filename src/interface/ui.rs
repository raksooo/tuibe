use super::component::{Backend, Component};
use crossterm::event::EventStream;
use tokio::select;
use tokio_stream::StreamExt;
use tui::Terminal;

#[derive(Clone)]
pub struct ProgramActions {
    quit_sender: flume::Sender<()>,
    redraw_sender: flume::Sender<()>,
}

#[allow(dead_code)]
impl ProgramActions {
    pub fn quit(&self) -> Result<(), flume::SendError<()>> {
        self.quit_sender.send(())
    }

    pub async fn quit_async(&self) -> Result<(), flume::SendError<()>> {
        self.quit_sender.send_async(()).await
    }

    pub fn redraw(&self) -> Result<(), flume::SendError<()>> {
        self.redraw_sender.send(())
    }

    pub async fn redraw_async(&self) -> Result<(), flume::SendError<()>> {
        self.redraw_sender.send_async(()).await
    }
}

pub async fn create<C, F>(terminal: &mut Terminal<Backend>, creator: F)
where
    C: Component,
    F: FnOnce(ProgramActions) -> C,
{
    let mut event_reader = EventStream::new();
    let (quit_sender, quit_receiver) = flume::unbounded();
    let (redraw_sender, redraw_receiver) = flume::unbounded();

    let program_actions = ProgramActions {
        quit_sender,
        redraw_sender,
    };

    let mut root = creator(program_actions.clone());
    program_actions
        .redraw_async()
        .await
        .expect("Failed to render");
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
