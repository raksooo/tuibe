use std::time::Duration;

use super::component::{Backend, Component};

use crossterm::event::EventStream;
use futures_timer::Delay;
use log::{debug, info};
use ratatui::Terminal;
use thiserror::Error;
use tokio::select;
use tokio_stream::StreamExt;

#[derive(Debug, Error)]
pub enum UiError {
    #[error("Failed to draw of component tree")]
    Draw(#[from] std::io::Error),

    #[error("Failed to send redraw message")]
    ReDraw(#[from] flume::SendError<UiMessage>),

    #[error("Failed to receive UiMessage")]
    MessageReceiver(#[from] flume::RecvError),
}

#[derive(Eq, PartialEq)]
pub enum UiMessage {
    Quit,
    Redraw,
}

pub async fn create<T: Component, F>(
    terminal: &mut Terminal<Backend>,
    creator: F,
) -> Result<(), UiError>
where
    F: FnOnce(flume::Sender<UiMessage>) -> T,
{
    let mut event_reader = EventStream::new();
    let (ui_sender, ui_receiver) = flume::unbounded();

    let mut root = creator(ui_sender.clone());
    ui_sender.send_async(UiMessage::Redraw).await?;

    info!("Starting event loop");
    loop {
        select! {
            event = event_reader.next() => {
                if let Some(Ok(event)) = event {
                    debug!("Received event: {:?}", event);
                    root.handle_event(event);
                }
            },

            // Delay recv by 8ms to make drawing cap at ~120 fps
            event = delayed_recv(&ui_receiver, 8) => match event? {
                UiMessage::Quit => break,
                UiMessage::Redraw => {
                    let drained = ui_receiver.drain();
                    if drained.len() > 0 {
                        debug!("Drained {} redraw messages", drained.len());

                        if drained.into_iter().any(|message| message == UiMessage::Quit) {
                            break;
                        }
                    }

                    debug!("Redrawing");
                    perform_draw(terminal, &mut root)?;
                }
            },
        };
    }
    info!("Exiting");

    Ok(())
}

async fn delayed_recv<T>(receiver: &flume::Receiver<T>, delay: u64) -> Result<T, flume::RecvError> {
    Delay::new(Duration::from_millis(delay)).await;
    receiver.recv_async().await
}

fn perform_draw<T: Component>(
    terminal: &mut Terminal<Backend>,
    root: &mut T,
) -> Result<(), UiError> {
    terminal.draw(|f| root.draw(f, f.area()))?;
    Ok(())
}
