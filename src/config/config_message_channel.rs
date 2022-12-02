use parking_lot::Mutex;
use tokio::sync::mpsc;

#[derive(Clone)]
pub enum ConfigMessage<T> {
    FinishedFetching,
    Clear,
    New(T),
    Remove(T),
    Error(String),
}

pub struct ConfigReceiver<T> {
    receiver: mpsc::UnboundedReceiver<ConfigMessage<T>>,
    items: Box<dyn Iterator<Item = T> + Send + Sync>,
}

impl<T: Send + Sync + 'static> ConfigReceiver<T> {
    pub fn new(items: Vec<T>, receiver: mpsc::UnboundedReceiver<ConfigMessage<T>>) -> Self {
        Self {
            items: Box::new(items.into_iter()),
            receiver,
        }
    }

    pub async fn recv(&mut self) -> Option<ConfigMessage<T>> {
        if let Some(item) = self.items.next() {
            Some(ConfigMessage::New(item))
        } else {
            self.receiver.recv().await
        }
    }
}

pub struct ConfigSender<T> {
    senders: Mutex<Vec<mpsc::UnboundedSender<ConfigMessage<T>>>>,
}

impl<T: Clone + Send + Sync + 'static> ConfigSender<T> {
    pub fn new() -> Self {
        Self {
            senders: Mutex::new(Vec::new()),
        }
    }

    pub fn send(&self, message: ConfigMessage<T>) {
        let mut senders = self.senders.lock();
        senders.retain(|sender| sender.send(message.clone()).is_ok());
    }

    pub fn subscribe(&self, items: Vec<T>) -> ConfigReceiver<T> {
        let (sender, receiver) = mpsc::unbounded_channel();
        let mut senders = self.senders.lock();
        senders.push(sender);
        ConfigReceiver::new(items, receiver)
    }
}
