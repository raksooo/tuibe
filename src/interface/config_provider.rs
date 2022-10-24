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
use parking_lot::Mutex;
use std::sync::Arc;
use tokio::sync::mpsc;
use tui::layout::Rect;

#[derive(Debug)]
pub enum ConfigProviderMsg {
    Reload,
}

pub struct ConfigProvider {
    program_tx: EventSender,
    config_tx: mpsc::Sender<ConfigProviderMsg>,
    app: Arc<Mutex<Box<dyn Component + Send>>>,
}

impl ConfigProvider {
    pub fn new(program_tx: EventSender) -> Self {
        let (config_tx, config_rx) = mpsc::channel(100);

        let mut config_provider = Self {
            program_tx: program_tx.clone(),
            config_tx,
            app: Arc::new(Mutex::new(Box::new(LoadingIndicator::new(
                program_tx.clone(),
            )))),
        };

        config_provider.init_configs();
        config_provider.listen_config_msg(config_rx);
        config_provider
    }

    fn listen_config_msg(&self, mut config_rx: mpsc::Receiver<ConfigProviderMsg>) {
        let program_tx = self.program_tx.clone();
        let config_tx = self.config_tx.clone();
        let app = Arc::clone(&self.app);

        tokio::spawn(async move {
            loop {
                match config_rx.recv().await.unwrap() {
                    ConfigProviderMsg::Reload => {
                        {
                            let mut app = app.lock();
                            *app = Box::new(LoadingIndicator::new(program_tx.clone()));
                        }
                        let _ = program_tx.send(UpdateEvent::Redraw).await;

                        Self::init_configs_impl(
                            program_tx.clone(),
                            config_tx.clone(),
                            Arc::clone(&app),
                        )
                        .await;
                        let _ = program_tx.send(UpdateEvent::Redraw).await;
                    }
                }
            }
        });
    }

    fn init_configs(&mut self) {
        let program_tx = self.program_tx.clone();
        let config_tx = self.config_tx.clone();
        let app = Arc::clone(&self.app);

        tokio::spawn(async move {
            Self::init_configs_impl(program_tx.clone(), config_tx, app).await;
            let _ = program_tx.send(UpdateEvent::Redraw).await;
        });
    }

    async fn init_configs_impl(
        program_tx: EventSender,
        config_tx: mpsc::Sender<ConfigProviderMsg>,
        app: Arc<Mutex<Box<dyn Component + Send>>>,
    ) {
        let new_app: Box<dyn Component + Send> = match Self::load_configs().await {
            Ok((common_config, config)) => Box::new(App::new(
                program_tx.clone(),
                config_tx,
                common_config,
                config,
            )),
            Err(_) => Box::new(Dialog::new("Something went wrong..")),
        };

        {
            let mut app = app.lock();
            *app = new_app;
        }
    }

    async fn load_configs() -> Result<(CommonConfigHandler, impl Config), ConfigError> {
        let common_config = CommonConfigHandler::load().await?;
        let config = RssConfigHandler::load().await?;
        let _ = config.fetch().await.unwrap()?;

        Ok((common_config, config))
    }
}

impl Component for ConfigProvider {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        let mut app = self.app.lock();
        app.draw(f, area);
    }

    fn handle_event(&mut self, event: Event) -> UpdateEvent {
        let mut app = self.app.lock();
        app.handle_event(event)
    }
}
