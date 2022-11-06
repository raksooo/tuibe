use super::{
    component::{Component, Frame},
    config::rss_view::RssConfigView,
    error_handler::{ErrorMsg, ErrorSenderExt},
    loading_indicator::LoadingIndicator,
    main_view::MainView,
};
use crate::config::{
    common::CommonConfigHandler, config::Config, error::ConfigError, rss::RssConfigHandler,
};
use crossterm::event::{Event, KeyCode};
use parking_lot::Mutex;
use std::sync::Arc;
use tui::layout::Rect;

#[derive(Debug)]
pub enum ConfigProviderMsg {
    Reload,
}

enum Child {
    Loading(LoadingIndicator),
    Main(MainView),
}

#[derive(Clone)]
pub struct ConfigProvider {
    inner: ConfigProviderInner,
}

#[derive(Clone)]
struct ConfigProviderInner {
    redraw_sender: flume::Sender<()>,
    error_sender: flume::Sender<ErrorMsg>,
    config_sender: flume::Sender<ConfigProviderMsg>,
    child: Arc<Mutex<Child>>,
}

impl ConfigProvider {
    pub fn new(redraw_sender: flume::Sender<()>, error_sender: flume::Sender<ErrorMsg>) -> Self {
        let (config_sender, config_receiver) = flume::bounded(1);

        let inner = ConfigProviderInner {
            redraw_sender: redraw_sender.clone(),
            error_sender,
            config_sender,
            child: Arc::new(Mutex::new(Child::Loading(LoadingIndicator::new(
                redraw_sender,
            )))),
        };

        let mut config_provider = Self { inner };
        config_provider.listen_config_msg(config_receiver);
        config_provider.init_configs();
        config_provider
    }

    fn listen_config_msg(&self, config_receiver: flume::Receiver<ConfigProviderMsg>) {
        let inner = self.inner.clone();

        tokio::spawn(async move {
            while let Ok(ConfigProviderMsg::Reload) = config_receiver.recv_async().await {
                Self::reload(inner.clone()).await;
            }
        });
    }

    fn init_configs(&mut self) {
        let inner = self.inner.clone();
        let redraw_sender = inner.redraw_sender.clone();

        tokio::spawn(async move {
            Self::init_configs_impl(inner).await;
            let _ = redraw_sender.send_async(()).await;
        });
    }

    async fn init_configs_impl(inner: ConfigProviderInner) {
        inner.error_sender.clone().run_or_send(
            Self::load_configs().await,
            false,
            move |(common_config, config)| {
                let videos = config.videos();

                let redraw_sender = inner.redraw_sender.clone();
                let error_sender = inner.error_sender.clone();
                let config_view_creator = |main_sender| {
                    RssConfigView::new(redraw_sender, error_sender, main_sender, config)
                };

                let mut child = inner.child.lock();
                *child = Child::Main(MainView::new(
                    inner.redraw_sender,
                    inner.error_sender,
                    inner.config_sender,
                    common_config,
                    videos,
                    config_view_creator,
                ));
            },
        );
    }

    async fn load_configs() -> Result<(CommonConfigHandler, RssConfigHandler), ConfigError> {
        let common_config = CommonConfigHandler::load().await?;
        let config = RssConfigHandler::load().await?;
        let _ = config.fetch().await.unwrap()?;

        Ok((common_config, config))
    }

    async fn reload(inner: ConfigProviderInner) {
        {
            let mut child = inner.child.lock();
            *child = Child::Loading(LoadingIndicator::new(inner.redraw_sender.clone()));
        }
        let _ = inner.redraw_sender.send_async(()).await;

        Self::init_configs_impl(inner.clone()).await;
        let _ = inner.redraw_sender.send_async(()).await;
    }
}

impl Component for ConfigProvider {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        let mut child = self.inner.child.lock();
        match *child {
            Child::Loading(ref mut loading_indicator) => &loading_indicator.draw(f, area),
            Child::Main(ref mut main_view) => &main_view.draw(f, area),
        };
    }

    fn handle_event(&mut self, event: Event) {
        let mut child = self.inner.child.lock();
        if let Child::Main(ref mut main_view) = *child {
            match event {
                Event::Key(event) if event.code == KeyCode::Char('r') => {
                    tokio::spawn(Self::reload(self.inner.clone()));
                }
                event => main_view.handle_event(event),
            }
        }
    }

    fn registered_events(&self) -> Vec<(String, String)> {
        let mut events = vec![];
        if let Child::Main(ref mut main_view) = *self.inner.child.lock() {
            events.push((String::from("r"), String::from("Reload")));
            events.append(&mut main_view.registered_events());
        }
        events
    }
}
