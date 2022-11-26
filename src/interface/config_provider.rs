use super::{
    component::{Component, Frame},
    config::rss_view::RssConfigView,
    loading_indicator::LoadingIndicatorActions,
    main_view::MainView,
};
use crate::config::{
    common::CommonConfigHandler, error::ConfigError, rss::RssConfigHandler, Config,
};

use crossterm::event::Event;
use parking_lot::Mutex;
use std::sync::Arc;
use tui::layout::Rect;

#[derive(Clone)]
pub struct ConfigProvider {
    actions: LoadingIndicatorActions,
    main_view: Arc<Mutex<Option<MainView>>>,
}

impl ConfigProvider {
    pub fn new(actions: LoadingIndicatorActions) -> Self {
        let mut config_provider = Self {
            actions,
            main_view: Arc::new(Mutex::new(None)),
        };

        config_provider.init_configs();
        config_provider
    }

    fn init_configs(&mut self) {
        let actions = self.actions.clone();
        let main_view = self.main_view.clone();

        tokio::spawn(async move {
            let init_result = Self::init_configs_impl(actions.clone(), main_view).await;
            actions.redraw_or_error_async(init_result, false).await;
        });
    }

    async fn init_configs_impl(
        actions: LoadingIndicatorActions,
        main_view: Arc<Mutex<Option<MainView>>>,
    ) -> Result<(), ConfigError> {
        let finished_loading = actions.loading();
        let (common_config, config) = Self::load_configs().await?;
        let config = Arc::new(config);

        let mut main_view = main_view.lock();
        *main_view = Some(MainView::new(
            actions,
            common_config,
            config.clone(),
            |actions| RssConfigView::new(actions, config),
        ));

        finished_loading();
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
        if let Some(ref mut main_view) = *self.main_view.lock() {
            main_view.draw(f, area);
        }
    }

    fn handle_event(&mut self, event: Event) {
        if let Some(ref mut main_view) = *self.main_view.lock() {
            main_view.handle_event(event);
        }
    }

    fn registered_events(&self) -> Vec<(String, String)> {
        let mut events = vec![];
        if let Some(ref mut main_view) = *self.main_view.lock() {
            events.append(&mut main_view.registered_events());
        }
        events
    }
}
