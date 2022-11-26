use super::Video;
use tokio::sync::broadcast;

#[derive(Clone)]
pub enum ConfigMessage {
    FinishedFetching,
    Clear,
    NewVideo(Video),
    RemoveVideosFrom(String),
    Error(String),
}

pub struct ConfigReceiver {
    receiver: broadcast::Receiver<ConfigMessage>,
    videos: Box<dyn Iterator<Item = Video> + Send + Sync>,
}

impl ConfigReceiver {
    pub fn new(videos: Vec<Video>, receiver: broadcast::Receiver<ConfigMessage>) -> Self {
        Self {
            videos: Box::new(videos.into_iter()),
            receiver,
        }
    }

    pub async fn recv(&mut self) -> Result<ConfigMessage, broadcast::error::RecvError> {
        if let Some(video) = self.videos.next() {
            Ok(ConfigMessage::NewVideo(video))
        } else {
            self.receiver.recv().await
        }
    }
}

pub struct ConfigSender(broadcast::Sender<ConfigMessage>);

impl ConfigSender {
    pub fn new(sender: broadcast::Sender<ConfigMessage>) -> Self {
        Self(sender)
    }

    pub fn send(&self, message: ConfigMessage) {
        let _ = self.0.send(message);
    }

    pub fn subscribe(&self, videos: Vec<Video>) -> ConfigReceiver {
        ConfigReceiver::new(videos, self.0.subscribe())
    }
}
