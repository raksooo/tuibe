use super::{
    component::{Component, Frame},
    config::rss_view::RssConfigView,
    error_handler::ErrorHandlerActions,
    loading_indicator::LoadingIndicator,
    main_view::MainView,
};
use crate::config::{
    common::CommonConfigHandler, error::ConfigError, rss::RssConfigHandler, Config,
};

use crossterm::event::Event;
use parking_lot::Mutex;
use std::sync::Arc;
use tui::layout::Rect;

enum Child {
    Loading(LoadingIndicator),
    Main(MainView),
}

#[derive(Clone)]
pub struct ConfigProvider {
    actions: ErrorHandlerActions,
    child: Arc<Mutex<Child>>,
}

impl ConfigProvider {
    pub fn new(actions: ErrorHandlerActions) -> Self {
        let mut config_provider = Self {
            actions: actions.clone(),
            child: Arc::new(Mutex::new(Child::Loading(LoadingIndicator::new(
                actions.redraw_fn(),
            )))),
        };

        config_provider.init_configs();
        config_provider
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
        actions: ErrorHandlerActions,
        child: Arc<Mutex<Child>>,
    ) -> Result<(), ConfigError> {
        let (common_config, config) = Self::load_configs().await?;
        let config = Arc::new(config);

        let mut child = child.lock();
        *child = Child::Main(MainView::new(
            actions,
            common_config,
            config.clone(),
            |actions| RssConfigView::new(actions, config),
        ));

        Ok(())
    }

    async fn load_configs() -> Result<(CommonConfigHandler, RssConfigHandler), ConfigError> {
        let common_config = CommonConfigHandler::load().await?;
        let config = RssConfigHandler::load().await?;
        Ok((common_config, config))
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
            main_view.handle_event(event);
        }
    }

    fn registered_events(&self) -> Vec<(String, String)> {
        let mut events = vec![];
        if let Child::Main(ref mut main_view) = *self.child.lock() {
            events.append(&mut main_view.registered_events());
        }
        events
    }
}
