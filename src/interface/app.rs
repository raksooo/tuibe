use crate::{
    common_config::CommonConfigHandler,
    config::{Config, ConfigData},
    interface::{
        component::{Component, EventSender, Frame, UpdateEvent},
        config_provider::ConfigProviderMsg,
        dialog::Dialog,
        feed::Feed,
        subscriptions::Subscriptions,
    },
};
use crossterm::event::{Event, KeyCode};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Debug)]
pub enum AppMsg {
    RemoveSubscription(String),
}

pub struct App {
    common_config: Arc<CommonConfigHandler>,
    config: Arc<Box<dyn Config + Send + Sync>>,

    tx: EventSender,
    config_tx: mpsc::Sender<ConfigProviderMsg>,
    app_tx: mpsc::Sender<AppMsg>,
    feed: Arc<Mutex<Box<dyn Component + Send>>>,
    subscriptions: Arc<Mutex<Option<Subscriptions>>>,
}

impl App {
    pub fn new<C>(
        tx: EventSender,
        config_tx: mpsc::Sender<ConfigProviderMsg>,
        common_config: CommonConfigHandler,
        config: C,
    ) -> Self
    where
        C: Config + Send + Sync + 'static,
    {
        let (app_tx, app_rx) = mpsc::channel(100);

        let feed = Self::create_feed(&common_config, &config.data());
        let mut app = Self {
            common_config: Arc::new(common_config),
            config: Arc::new(Box::new(config)),

            tx: tx.clone(),
            config_tx,
            app_tx: app_tx.clone(),
            feed: Arc::new(Mutex::new(Box::new(feed))),
            subscriptions: Arc::new(Mutex::new(None)),
        };

        app.init_channel(app_rx);
        app
    }

    fn init_channel(&mut self, app_rx: mpsc::Receiver<AppMsg>) {
        let tx = self.tx.clone();
        let common_config = Arc::clone(&self.common_config);
        let config = Arc::clone(&self.config);
        let feed = Arc::clone(&self.feed);
        let subscriptions = Arc::clone(&self.subscriptions);

        tokio::spawn(async move {
            let _ = Self::listen_app_msg(
                app_rx,
                tx,
                Arc::clone(&common_config),
                Arc::clone(&config),
                Arc::clone(&feed),
                Arc::clone(&subscriptions),
            )
            .await;

            let mut feed = feed.lock().unwrap();
            *feed = Box::new(Dialog::new("Something went wrong.."));
        });
    }

    async fn listen_app_msg(
        mut app_rx: mpsc::Receiver<AppMsg>,
        tx: EventSender,
        common_config: Arc<CommonConfigHandler>,
        config: Arc<Box<dyn Config + Send + Sync>>,
        feed: Arc<Mutex<Box<dyn Component + Send>>>,
        subscriptions: Arc<Mutex<Option<Subscriptions>>>,
    ) -> Result<(), ()> {
        loop {
            let msg = app_rx.recv().await.ok_or(())?;
            match msg {
                AppMsg::RemoveSubscription(subscription) => {
                    let data = config
                        .remove_subscription(subscription)
                        .await
                        .unwrap()
                        .map_err(|_| ())?;
                    Self::propagate_data(
                        tx.clone(),
                        Arc::clone(&common_config),
                        Arc::clone(&feed),
                        Arc::clone(&subscriptions),
                        data,
                    )
                    .await;
                }
            }
        }
    }

    async fn propagate_data(
        tx: EventSender,
        common_config: Arc<CommonConfigHandler>,
        feed: Arc<Mutex<Box<dyn Component + Send>>>,
        subscriptions: Arc<Mutex<Option<Subscriptions>>>,
        data: ConfigData,
    ) {
        {
            let mut feed = feed.lock().unwrap();
            *feed = Box::new(Self::create_feed(&*common_config, &data));
        }

        if let Some(ref mut subscriptions) = *subscriptions.lock().unwrap() {
            subscriptions.update_channels(data.channels);
        }

        let _ = tx.send(UpdateEvent::Redraw).await;
    }

    fn toggle_subscriptions(&self) -> UpdateEvent {
        let mut subscriptions = self.subscriptions.lock().unwrap();
        if subscriptions.is_some() {
            *subscriptions = None;
        } else {
            *subscriptions = Some(Subscriptions::new(
                self.tx.clone(),
                self.app_tx.clone(),
                self.config.data().channels,
            ));
        }

        UpdateEvent::Redraw
    }

    fn reload(&self) -> UpdateEvent {
        let config_tx = self.config_tx.clone();
        tokio::spawn(async move {
            let _ = config_tx.send(ConfigProviderMsg::Reload).await;
        });
        UpdateEvent::None
    }

    fn create_feed(common_config: &CommonConfigHandler, data: &ConfigData) -> Feed {
        let last_played_timestamp = common_config.config().last_played_timestamp;
        Feed::new(
            data.videos.clone().into_iter().collect(),
            last_played_timestamp,
        )
    }
}

impl Component for App {
    fn draw(&mut self, f: &mut Frame, size: Rect) {
        let mut subscriptions = self.subscriptions.lock().unwrap();
        let subscriptions_numerator = subscriptions.as_ref().map_or(0, |_| 1);

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Ratio(subscriptions_numerator, 2),
                    Constraint::Ratio(2 - subscriptions_numerator, 2),
                ]
                .as_ref(),
            )
            .split(size);

        if let Some(ref mut subscriptions) = *subscriptions {
            subscriptions.draw(f, chunks[0]);
        }

        let mut feed = self.feed.lock().unwrap();
        feed.draw(f, chunks[1]);
    }

    fn handle_event(&mut self, event: Event) -> UpdateEvent {
        if let Event::Key(event) = event {
            match event.code {
                KeyCode::Char('q') => return UpdateEvent::Quit,
                KeyCode::Char('s') => return self.toggle_subscriptions(),
                KeyCode::Char('r') => return self.reload(),
                _ => (),
            }
        }

        let mut subscriptions = self.subscriptions.lock().unwrap();
        if let Some(ref mut subscriptions) = *subscriptions {
            subscriptions.handle_event(event)
        } else {
            let mut feed = self.feed.lock().unwrap();
            feed.handle_event(event)
        }
    }
}
