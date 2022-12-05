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
    #[error(display = "Failed to draw of component tree")]
    Draw(#[error(from)] std::io::Error),

    #[error(display = "Failed to send redraw message")]
    ReDraw(#[error(from)] flume::SendError<UiMessage>),

    #[error(display = "Failed to receive UiMessage")]
    MessageReceiver(#[error(from)] flume::RecvError),
}

#[derive(Eq, PartialEq)]
pub enum UiMessage {
    Quit,
    Redraw,
}

pub async fn create<C, F>(terminal: &mut Terminal<Backend>, creator: F) -> Result<(), UiError>
where
    C: Component,
    F: FnOnce(flume::Sender<UiMessage>) -> C,
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

            event = ui_receiver.recv_async() => match event? {
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
