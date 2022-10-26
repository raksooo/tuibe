use tokio::sync::mpsc::Sender;

pub trait SenderExt<T> {
    fn send_sync(&self, value: T);
}

impl<T: Send + 'static> SenderExt<T> for Sender<T> {
    fn send_sync(&self, value: T) {
        let sender = self.clone();
        tokio::spawn(async move {
            let _ = sender.send(value).await;
        });
    }
}
