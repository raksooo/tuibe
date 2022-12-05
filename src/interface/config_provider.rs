use super::{
    backend::rss_view::RssBackendView,
    component::{Component, Frame},
    main_view::MainView,
    status_label::{StatusLabelActions, LOADING_STRING},
};
use crate::backend::{rss::RssBackend, Backend, BackendError};
use crate::config::ConfigHandler;

use crossterm::event::Event;
use parking_lot::Mutex;
use std::sync::Arc;
use tui::layout::Rect;

#[derive(Clone)]
pub struct ConfigProvider {
    actions: StatusLabelActions,
    main_view: Arc<Mutex<Option<MainView>>>,
}

impl ConfigProvider {
    pub fn new(actions: StatusLabelActions) -> Self {
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
        actions: StatusLabelActions,
        main_view: Arc<Mutex<Option<MainView>>>,
    ) -> Result<(), BackendError> {
        let finished_loading = actions.show_label(LOADING_STRING);
        let (config, backend) = Self::load_configs().await?;
        let backend = Arc::new(backend);

        let mut main_view = main_view.lock();
        *main_view = Some(MainView::new(actions, config, backend.clone(), |actions| {
            RssBackendView::new(actions, backend)
        }));

        finished_loading();
        Ok(())
    }

    async fn load_configs() -> Result<(ConfigHandler, RssBackend), BackendError> {
        let config = ConfigHandler::load().await?;
        let backend = RssBackend::load().await?;
        Ok((config, backend))
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
