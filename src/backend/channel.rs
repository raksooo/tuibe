use parking_lot::Mutex;
use tokio::sync::mpsc;

#[derive(Clone)]
pub enum BackendMessage<T> {
    FinishedFetching,
    Clear,
    New(T),
    Remove(T),
    Error(String),
}

pub struct BackendReceiver<T> {
    receiver: mpsc::UnboundedReceiver<BackendMessage<T>>,
    items: Box<dyn Iterator<Item = T> + Send + Sync>,
}

impl<T: Send + Sync + 'static> BackendReceiver<T> {
    pub fn new(items: Vec<T>, receiver: mpsc::UnboundedReceiver<BackendMessage<T>>) -> Self {
        Self {
            items: Box::new(items.into_iter()),
            receiver,
        }
    }

    pub async fn recv(&mut self) -> Option<BackendMessage<T>> {
        if let Some(item) = self.items.next() {
            Some(BackendMessage::New(item))
        } else {
            self.receiver.recv().await
        }
    }
}

pub struct BackendSender<T> {
    senders: Mutex<Vec<mpsc::UnboundedSender<BackendMessage<T>>>>,
}

impl<T: Clone + Send + Sync + 'static> BackendSender<T> {
    pub fn new() -> Self {
        Self {
            senders: Mutex::new(Vec::new()),
        }
    }

    pub fn send(&self, message: BackendMessage<T>) {
        let mut senders = self.senders.lock();
        senders.retain(|sender| sender.send(message.clone()).is_ok());
    }

    pub fn subscribe(&self, items: Vec<T>) -> BackendReceiver<T> {
        let (sender, receiver) = mpsc::unbounded_channel();
        let mut senders = self.senders.lock();
        senders.push(sender);
        BackendReceiver::new(items, receiver)
    }
}
