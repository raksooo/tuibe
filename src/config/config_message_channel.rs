use super::Video;
use parking_lot::Mutex;
use tokio::sync::mpsc;

#[derive(Clone)]
pub enum ConfigMessage {
    FinishedFetching,
    Clear,
    NewVideo(Video),
    RemoveVideosFrom(String),
    Error(String),
}

pub struct ConfigReceiver {
    receiver: mpsc::UnboundedReceiver<ConfigMessage>,
    videos: Box<dyn Iterator<Item = Video> + Send + Sync>,
}

impl ConfigReceiver {
    pub fn new(videos: Vec<Video>, receiver: mpsc::UnboundedReceiver<ConfigMessage>) -> Self {
        Self {
            videos: Box::new(videos.into_iter()),
            receiver,
        }
    }

    pub async fn recv(&mut self) -> Option<ConfigMessage> {
        if let Some(video) = self.videos.next() {
            Some(ConfigMessage::NewVideo(video))
        } else {
            self.receiver.recv().await
        }
    }
}

pub struct ConfigSender {
    senders: Mutex<Vec<mpsc::UnboundedSender<ConfigMessage>>>,
}

impl ConfigSender {
    pub fn new() -> Self {
        Self {
            senders: Mutex::new(Vec::new()),
        }
    }

    pub fn send(&self, message: ConfigMessage) {
        let mut senders = self.senders.lock();
        senders.retain(|sender| sender.send(message.clone()).is_ok());
    }

    pub fn subscribe(&self, videos: Vec<Video>) -> ConfigReceiver {
        let (sender, receiver) = mpsc::unbounded_channel();
        let mut senders = self.senders.lock();
        senders.push(sender);
        ConfigReceiver::new(videos, receiver)
    }
}
