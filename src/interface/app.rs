use crate::{
    common_config::CommonConfigHandler,
    config::{Config, ConfigData},
    interface::{
        component::{Component, EventSender, Frame, UpdateEvent},
        dialog::Dialog,
        feed::Feed,
        subscriptions::Subscriptions,
    },
};
use crossterm::event::{Event, KeyCode};
use std::sync::{mpsc, Arc, Mutex};
use tui::layout::{Constraint, Direction, Layout, Rect};

pub enum AppMsg {
    Reload,
    RemoveSubscription(String),
}

pub struct App {
    common_config: Arc<CommonConfigHandler>,
    config: Arc<Box<dyn Config + Send + Sync>>,

    tx: EventSender,
    app_tx: mpsc::Sender<AppMsg>,
    feed: Arc<Mutex<Box<dyn Component + Send>>>,
    subscriptions: Arc<Mutex<Option<Subscriptions>>>,
}

impl App {
    pub fn new<C>(tx: EventSender, common_config: CommonConfigHandler, config: C) -> Self
    where
        C: Config + Send + Sync + 'static,
    {
        let (app_tx, app_rx) = mpsc::channel();

        let feed = Self::create_feed(app_tx.clone(), &common_config, &config.data());
        let mut app = Self {
            common_config: Arc::new(common_config),
            config: Arc::new(Box::new(config)),

            tx: tx.clone(),
            app_tx: app_tx.clone(),
            feed: Arc::new(Mutex::new(Box::new(feed))),
            subscriptions: Arc::new(Mutex::new(None)),
        };

        app.init_channel(app_rx, app_tx);
        app
    }

    fn init_channel(&mut self, app_rx: mpsc::Receiver<AppMsg>, app_tx: mpsc::Sender<AppMsg>) {
        let tx = self.tx.clone();
        let common_config = Arc::clone(&self.common_config);
        let config = Arc::clone(&self.config);
        let feed = Arc::clone(&self.feed);
        let subscriptions = Arc::clone(&self.subscriptions);

        tokio::spawn(async move {
            let _ = Self::listen_app_msg(
                app_rx,
                app_tx,
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
        app_rx: mpsc::Receiver<AppMsg>,
        app_tx: mpsc::Sender<AppMsg>,
        tx: EventSender,
        common_config: Arc<CommonConfigHandler>,
        config: Arc<Box<dyn Config + Send + Sync>>,
        feed: Arc<Mutex<Box<dyn Component + Send>>>,
        subscriptions: Arc<Mutex<Option<Subscriptions>>>,
    ) -> Result<(), ()> {
        loop {
            let app_tx = app_tx.clone();
            let msg = app_rx.recv().map_err(|_| ())?;
            match msg {
                AppMsg::RemoveSubscription(subscription) => {
                    let data = config
                        .remove_subscription(subscription)
                        .await
                        .unwrap()
                        .map_err(|_| ())?;
                    Self::propagate_data(
                        tx.clone(),
                        app_tx,
                        Arc::clone(&common_config),
                        Arc::clone(&feed),
                        Arc::clone(&subscriptions),
                        data,
                    ).await;
                }
                AppMsg::Reload => {
                    let data_rx = config.fetch();
                    let _ = tx.send(UpdateEvent::Redraw).await;

                    let data = data_rx
                        .await
                        .unwrap()
                        .map_err(|_| ())?;

                    Self::propagate_data(
                        tx.clone(),
                        app_tx,
                        Arc::clone(&common_config),
                        Arc::clone(&feed),
                        Arc::clone(&subscriptions),
                        data,
                    ).await;
                }
            }
        }
    }

    async fn propagate_data(
        tx: EventSender,
        app_tx: mpsc::Sender<AppMsg>,
        common_config: Arc<CommonConfigHandler>,
        feed: Arc<Mutex<Box<dyn Component + Send>>>,
        subscriptions: Arc<Mutex<Option<Subscriptions>>>,
        data: ConfigData,
    ) {
        {
            let mut feed = feed.lock().unwrap();
            *feed = Box::new(Self::create_feed(app_tx, &*common_config, &data));
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

    fn create_feed(
        app_tx: mpsc::Sender<AppMsg>,
        common_config: &CommonConfigHandler,
        data: &ConfigData,
    ) -> Feed {
        let last_played_timestamp = common_config.config().last_played_timestamp;
        Feed::new(
            app_tx,
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
