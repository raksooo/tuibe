use super::{
    component::{Component, Frame},
    config::rss_view::RssConfigView,
    error_handler::{ErrorHandlerActions, ErrorMessage},
    loading_indicator::LoadingIndicator,
    main_view::MainView,
};
use crate::config::{
    common::CommonConfigHandler, error::ConfigError, rss::RssConfigHandler, Config,
};

use crossterm::event::{Event, KeyCode};
use delegate::delegate;
use parking_lot::Mutex;
use std::{fmt::Display, sync::Arc};
use tui::layout::Rect;

#[derive(Debug)]
pub enum ConfigProviderMessage {
    Reload,
}

#[derive(Clone)]
pub struct ConfigProviderActions {
    error_handler_actions: ErrorHandlerActions,
    config_sender: flume::Sender<ConfigProviderMessage>,
}

#[allow(dead_code)]
impl ConfigProviderActions {
    pub fn reload_config(&self) {
        self.error_handler_actions
            .handle_result(self.config_sender.send(ConfigProviderMessage::Reload));
    }

    pub async fn reload_config_async(&self) {
        self.error_handler_actions
            .handle_result_async(
                self.config_sender
                    .send_async(ConfigProviderMessage::Reload)
                    .await,
            )
            .await;
    }

    delegate! {
        to self.error_handler_actions {
            pub fn error(&self, error: ErrorMessage);
            pub async fn error_async(&self, error: ErrorMessage);
            pub fn redraw_or_error<T, E: Display>(&self, result: Result<T, E>, ignorable: bool);
            pub async fn redraw_or_error_async<T, E: Display>(&self, result: Result<T, E>, ignorable: bool);
            pub fn handle_result<T, E: Display>(&self, result: Result<T, E>);
            pub async fn handle_result_async<T, E: Display>(&self, result: Result<T, E>);
            pub fn redraw(&self);
            pub async fn redraw_async(&self);
            pub fn redraw_fn(&self) -> impl Fn();
        }
    }
}

enum Child {
    Loading(LoadingIndicator),
    Main(MainView),
}

#[derive(Clone)]
pub struct ConfigProvider {
    actions: ConfigProviderActions,
    child: Arc<Mutex<Child>>,
}

impl ConfigProvider {
    pub fn new(error_handler_actions: ErrorHandlerActions) -> Self {
        let (config_sender, config_receiver) = flume::bounded(1);
        let actions = ConfigProviderActions {
            error_handler_actions,
            config_sender,
        };

        let mut config_provider = Self {
            actions: actions.clone(),
            child: Arc::new(Mutex::new(Child::Loading(LoadingIndicator::new(
                actions.redraw_fn(),
            )))),
        };

        config_provider.listen_config_msg(config_receiver);
        config_provider.init_configs();
        config_provider
    }

    fn listen_config_msg(&self, config_receiver: flume::Receiver<ConfigProviderMessage>) {
        let actions = self.actions.clone();
        let child = self.child.clone();

        tokio::spawn(async move {
            while let Ok(ConfigProviderMessage::Reload) = config_receiver.recv_async().await {
                Self::reload(actions.clone(), child.clone()).await;
            }
        });
    }

    fn init_configs(&mut self) {
        let actions = self.actions.clone();
        let child = self.child.clone();

        tokio::spawn(async move {
            let init_result = Self::init_configs_impl(actions.clone(), child).await;
            actions.redraw_or_error_async(init_result, false).await;
        });
    }

    async fn init_configs_impl(
        actions: ConfigProviderActions,
        child: Arc<Mutex<Child>>,
    ) -> Result<(), ConfigError> {
        let (common_config, config) = Self::load_configs().await?;

        let mut child = child.lock();
        *child = Child::Main(MainView::new(
            actions,
            common_config,
            config.videos(),
            |actions| RssConfigView::new(actions, config),
        ));

        Ok(())
    }

    async fn load_configs() -> Result<(CommonConfigHandler, RssConfigHandler), ConfigError> {
        let common_config = CommonConfigHandler::load().await?;
        let config = RssConfigHandler::load().await?;
        config.fetch().await?;

        Ok((common_config, config))
    }

    async fn reload(actions: ConfigProviderActions, child: Arc<Mutex<Child>>) {
        {
            let mut child = child.lock();
            *child = Child::Loading(LoadingIndicator::new(actions.redraw_fn()));
        }
        actions.redraw_async().await;

        let init_result = Self::init_configs_impl(actions.clone(), child).await;
        actions.redraw_or_error_async(init_result, false).await;
    }
}

impl Component for ConfigProvider {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        let mut child = self.child.lock();
        match *child {
            Child::Loading(ref mut loading_indicator) => &loading_indicator.draw(f, area),
            Child::Main(ref mut main_view) => &main_view.draw(f, area),
        };
    }

    fn handle_event(&mut self, event: Event) {
        let mut child = self.child.lock();
        if let Child::Main(ref mut main_view) = *child {
            match event {
                Event::Key(event) if event.code == KeyCode::Char('r') => {
                    tokio::spawn(Self::reload(self.actions.clone(), self.child.clone()));
                }
                event => main_view.handle_event(event),
            }
        }
    }

    fn registered_events(&self) -> Vec<(String, String)> {
        let mut events = vec![];
        if let Child::Main(ref mut main_view) = *self.child.lock() {
            events.push((String::from("r"), String::from("Reload")));
            events.append(&mut main_view.registered_events());
        }
        events
    }
}
