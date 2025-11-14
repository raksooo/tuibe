use super::{
    actions::Actions,
    component::{Component, Frame},
    main_view::MainView,
    status_label::LOADING_STRING,
};
use crate::backend::{Backend, BackendError, rss::RssBackend};
use crate::config::ConfigHandler;

use crossterm::event::Event;
use parking_lot::Mutex;
use ratatui::layout::{Rect, Size};
use std::sync::Arc;

#[derive(Clone)]
pub struct BackendProvider {
    actions: Actions,
    main_view: Arc<Mutex<Option<MainView>>>,
}

impl BackendProvider {
    pub fn new(actions: Actions) -> Self {
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
        actions: Actions,
        main_view: Arc<Mutex<Option<MainView>>>,
    ) -> Result<(), BackendError> {
        let finished_loading = actions.show_label(LOADING_STRING);
        let config = ConfigHandler::load().await?;
        let backend = Arc::new(RssBackend::load().await?);

        let mut main_view = main_view.lock();
        *main_view = Some(MainView::new(actions, config, backend.clone()));

        finished_loading();
        Ok(())
    }
}

impl Component for BackendProvider {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        if let Some(ref mut main_view) = *self.main_view.lock() {
            main_view.draw(f, area);
        }
    }

    fn handle_event(&mut self, event: Event, size: Option<Size>) {
        if let Some(ref mut main_view) = *self.main_view.lock() {
            main_view.handle_event(event, size);
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
