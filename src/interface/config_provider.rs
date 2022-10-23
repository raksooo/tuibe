use crate::{
    common_config::CommonConfigHandler,
    config::Config,
    error::ConfigError,
    interface::{
        app::App,
        component::{Component, EventSender, Frame, UpdateEvent},
        dialog::Dialog,
        loading_indicator::LoadingIndicator,
    },
    rss_config::RssConfigHandler,
};
use crossterm::event::Event;
use std::sync::{Arc, Mutex};
use tui::layout::Rect;

pub struct ConfigProvider {
    tx: EventSender,
    app: Arc<Mutex<Box<dyn Component + Send>>>,
}

impl ConfigProvider {
    pub fn new(tx: EventSender) -> Self {
        let mut config_provider = Self {
            tx: tx.clone(),
            app: Arc::new(Mutex::new(Box::new(LoadingIndicator::new(tx.clone())))),
        };

        config_provider.init();
        config_provider
    }

    fn init(&mut self) {
        let tx = self.tx.clone();
        let app = Arc::clone(&self.app);

        tokio::spawn(async move {
            let new_app: Box<dyn Component + Send> = match Self::init_impl().await {
                Ok((common_config, config)) => {
                    Box::new(App::new(tx.clone(), common_config, config))
                }
                Err(_) => Box::new(Dialog::new("Something went wrong..")),
            };

            {
                let mut app = app.lock().unwrap();
                *app = new_app;
            }

            let _ = tx.send(UpdateEvent::Redraw).await;
        });
    }

    async fn init_impl() -> Result<(CommonConfigHandler, impl Config), ConfigError> {
        let common_config = CommonConfigHandler::load().await?;
        let config = RssConfigHandler::load().await?;
        let _ = config.fetch().await.unwrap()?;

        Ok((common_config, config))
    }
}

impl Component for ConfigProvider {
    fn draw(&mut self, f: &mut Frame, size: Rect) {
        let mut app = self.app.lock().unwrap();
        app.draw(f, size);
    }

    fn handle_event(&mut self, event: Event) -> UpdateEvent {
        let mut app = self.app.lock().unwrap();
        app.handle_event(event)
    }
}
