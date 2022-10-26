use super::{
    component::{Component, EventSender, Frame, UpdateEvent},
    config_provider::ConfigProviderMsg,
    dialog::Dialog,
    feed::Feed,
    subscriptions::Subscriptions,
};
use crate::{
    config::{
        common::CommonConfigHandler,
        config::{Config, ConfigData},
    },
    sender_ext::SenderExt,
};
use crossterm::event::{Event, KeyCode};
use parking_lot::Mutex;
use std::sync::Arc;
use tokio::sync::mpsc;
use tui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Debug)]
pub enum AppMsg {
    RemoveSubscription(String),
}

struct AppInner {
    common_config: CommonConfigHandler,
    config: Box<dyn Config + Send + Sync>,

    feed: Mutex<Box<dyn Component + Send>>,
    subscriptions: Mutex<Option<Subscriptions>>,
}

pub struct App {
    inner: Arc<AppInner>,

    program_tx: EventSender,
    config_tx: mpsc::Sender<ConfigProviderMsg>,
    app_tx: mpsc::Sender<AppMsg>,
}

impl App {
    pub fn new<C>(
        program_tx: EventSender,
        config_tx: mpsc::Sender<ConfigProviderMsg>,
        common_config: CommonConfigHandler,
        config: C,
    ) -> Self
    where
        C: Config + Send + Sync + 'static,
    {
        let (app_tx, app_rx) = mpsc::channel(100);

        let feed = Self::create_feed(&common_config, &config.data());

        let inner = AppInner {
            common_config: common_config,
            config: Box::new(config),
            feed: Mutex::new(Box::new(feed)),
            subscriptions: Mutex::new(None),
        };

        let mut app = Self {
            inner: Arc::new(inner),
            program_tx: program_tx.clone(),
            config_tx,
            app_tx: app_tx.clone(),
        };

        app.init_channel(app_rx);
        app
    }

    fn init_channel(&mut self, app_rx: mpsc::Receiver<AppMsg>) {
        let program_tx = self.program_tx.clone();
        let inner = Arc::clone(&self.inner);

        tokio::spawn(async move {
            let _ = Self::listen_app_msg(app_rx, program_tx, Arc::clone(&inner)).await;
            let mut feed = inner.feed.lock();
            *feed = Box::new(Dialog::new("Something went wrong.."));
        });
    }

    async fn listen_app_msg(
        mut app_rx: mpsc::Receiver<AppMsg>,
        program_tx: EventSender,
        inner: Arc<AppInner>,
    ) -> Result<(), ()> {
        loop {
            let msg = app_rx.recv().await.ok_or(())?;
            match msg {
                AppMsg::RemoveSubscription(subscription) => {
                    let data = inner
                        .config
                        .remove_subscription(subscription)
                        .await
                        .unwrap()
                        .map_err(|_| ())?;
                    Self::propagate_data(program_tx.clone(), Arc::clone(&inner), data).await;
                }
            }
        }
    }

    async fn propagate_data(program_tx: EventSender, inner: Arc<AppInner>, data: ConfigData) {
        {
            let mut feed = inner.feed.lock();
            *feed = Box::new(Self::create_feed(&inner.common_config, &data));
        }

        if let Some(ref mut subscriptions) = *inner.subscriptions.lock() {
            subscriptions.update_channels(data.channels);
        }

        let _ = program_tx.send(UpdateEvent::Redraw).await;
    }

    fn toggle_subscriptions(&self) -> UpdateEvent {
        let mut subscriptions = self.inner.subscriptions.lock();
        if subscriptions.is_some() {
            *subscriptions = None;
        } else {
            *subscriptions = Some(Subscriptions::new(
                self.program_tx.clone(),
                self.app_tx.clone(),
                self.inner.config.data().channels,
            ));
        }

        UpdateEvent::Redraw
    }

    fn reload(&self) -> UpdateEvent {
        self.config_tx.send_sync(ConfigProviderMsg::Reload);
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
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        let mut subscriptions = self.inner.subscriptions.lock();
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
            .split(area);

        if let Some(ref mut subscriptions) = *subscriptions {
            subscriptions.draw(f, chunks[0]);
        }

        let mut feed = self.inner.feed.lock();
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

        let mut subscriptions = self.inner.subscriptions.lock();
        if let Some(ref mut subscriptions) = *subscriptions {
            subscriptions.handle_event(event)
        } else {
            let mut feed = self.inner.feed.lock();
            feed.handle_event(event)
        }
    }
}
