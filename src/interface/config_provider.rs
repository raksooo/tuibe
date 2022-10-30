use super::{
    component::{Component, EventSender, Frame, UpdateEvent},
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
use tokio::sync::mpsc;
use tui::layout::Rect;

#[derive(Debug)]
pub enum ConfigProviderMsg {
    Reload,
}

pub struct ConfigProvider {
    program_sender: EventSender,
    error_sender: mpsc::Sender<ErrorMsg>,
    config_sender: mpsc::Sender<ConfigProviderMsg>,
    main_view: Arc<Mutex<Box<dyn Component + Send>>>,
}

impl ConfigProvider {
    pub fn new(program_sender: EventSender, error_sender: mpsc::Sender<ErrorMsg>) -> Self {
        let (config_sender, config_receiver) = mpsc::channel(100);

        let mut config_provider = Self {
            program_sender: program_sender.clone(),
            error_sender,
            config_sender,
            main_view: Arc::new(Mutex::new(Box::new(LoadingIndicator::new(
                program_sender.clone(),
            )))),
        };

        config_provider.init_configs();
        config_provider.listen_config_msg(config_receiver);
        config_provider
    }

    fn listen_config_msg(&self, mut config_receiver: mpsc::Receiver<ConfigProviderMsg>) {
        let program_sender = self.program_sender.clone();
        let error_sender = self.error_sender.clone();
        let config_sender = self.config_sender.clone();
        let main_view = Arc::clone(&self.main_view);

        tokio::spawn(async move {
            loop {
                match config_receiver.recv().await.unwrap() {
                    ConfigProviderMsg::Reload => {
                        Self::reload(
                            program_sender.clone(),
                            error_sender.clone(),
                            config_sender.clone(),
                            Arc::clone(&main_view),
                        )
                        .await
                    }
                }
            }
        });
    }

    fn init_configs(&mut self) {
        let program_sender = self.program_sender.clone();
        let error_sender = self.error_sender.clone();
        let config_sender = self.config_sender.clone();
        let main_view = Arc::clone(&self.main_view);

        tokio::spawn(async move {
            Self::init_configs_impl(
                program_sender.clone(),
                error_sender,
                config_sender,
                main_view,
            )
            .await;
            let _ = program_sender.send(UpdateEvent::Redraw).await;
        });
    }

    async fn init_configs_impl(
        program_sender: EventSender,
        error_sender: mpsc::Sender<ErrorMsg>,
        config_sender: mpsc::Sender<ConfigProviderMsg>,
        main_view: Arc<Mutex<Box<dyn Component + Send>>>,
    ) {
        error_sender.clone().run_or_send(
            Self::load_configs().await,
            false,
            move |(common_config, config)| {
                let mut main_view = main_view.lock();
                *main_view = Box::new(MainView::new(
                    program_sender.clone(),
                    error_sender.clone(),
                    config_sender,
                    common_config,
                    config.videos(),
                    |main_sender| {
                        RssConfigView::new(program_sender, error_sender, main_sender, config)
                    },
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

    async fn reload(
        program_sender: EventSender,
        error_sender: mpsc::Sender<ErrorMsg>,
        config_sender: mpsc::Sender<ConfigProviderMsg>,
        main_view: Arc<Mutex<Box<dyn Component + Send>>>,
    ) {
        {
            let mut main_view = main_view.lock();
            *main_view = Box::new(LoadingIndicator::new(program_sender.clone()));
        }
        let _ = program_sender.send(UpdateEvent::Redraw).await;

        Self::init_configs_impl(
            program_sender.clone(),
            error_sender.clone(),
            config_sender.clone(),
            Arc::clone(&main_view),
        )
        .await;
        let _ = program_sender.send(UpdateEvent::Redraw).await;
    }
}

impl Component for ConfigProvider {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        let mut main_view = self.main_view.lock();
        main_view.draw(f, area);
    }

    fn handle_event(&mut self, event: Event) {
        if let Event::Key(event) = event {
            match event.code {
                KeyCode::Char('r') => {
                    let program_sender = self.program_sender.clone();
                    let error_sender = self.error_sender.clone();
                    let config_sender = self.config_sender.clone();
                    let main_view = Arc::clone(&self.main_view);
                    tokio::spawn(async move {
                        Self::reload(program_sender, error_sender, config_sender, main_view).await;
                    });
                    return;
                }
                _ => (),
            }
        }

        let mut main_view = self.main_view.lock();
        main_view.handle_event(event);
    }
}
