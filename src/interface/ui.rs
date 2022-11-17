use std::time::Duration;

use super::component::{Backend, Component};

use crossterm::event::EventStream;
use err_derive::Error;
use futures_timer::Delay;
use log::{debug, info};
use tokio::select;
use tokio_stream::StreamExt;
use tui::Terminal;

#[derive(Debug, Error)]
pub enum UiError {
    #[error(display = "Failed to perform initial draw of component tree")]
    Draw(#[error(from)] std::io::Error),
    #[error(display = "Failed to send redraw message")]
    ReDraw(#[error(from)] flume::SendError<()>),
}

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

pub async fn create<C, F>(terminal: &mut Terminal<Backend>, creator: F) -> Result<(), UiError>
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
    program_actions.redraw_async().await?;

    info!("Starting event loop");
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
                    let drained = redraw_receiver.drain();
                    if drained.len() > 0 {
                        debug!("Drained {} redraw messages", drained.len());
                    }

                    perform_draw(terminal, &mut root)?;

                    // Wait a few milliseconds to prevent to many consecutive redraws
                    Delay::new(Duration::from_millis(5)).await;
                }
            },
        };
    }
    info!("Exiting");

    Ok(())
}

fn perform_draw(
    terminal: &mut Terminal<Backend>,
    root: &mut impl Component,
) -> Result<(), UiError> {
    terminal.draw(|f| root.draw(f, f.size()))?;
    Ok(())
}
