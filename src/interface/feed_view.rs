use super::{
    actions::Actions,
    component::{Component, Frame},
    status_label::LOADING_STRING,
    video_list::VideoList,
};
use crate::backend::{channel::BackendMessage, Backend, Video};
use crate::config::ConfigHandler;

use crossterm::event::{Event, KeyCode};
use parking_lot::Mutex;
use std::{env, process::Stdio, sync::Arc};
use tokio::process::Command;
use tui::{
    layout::Rect,
    style::{Color, Style},
    widgets::Block,
};

pub struct FeedView {
    actions: Actions,
    config: Arc<ConfigHandler>,
    loading_id: Arc<Mutex<Option<usize>>>,
    video_list: Arc<Mutex<VideoList>>,
}

impl FeedView {
    pub fn new(
        actions: Actions,
        config: ConfigHandler,
        backend: Arc<impl Backend + Send + Sync + 'static>,
    ) -> Self {
        let feed_view = Self {
            actions,
            config: Arc::new(config),
            loading_id: Default::default(),
            video_list: Arc::new(Mutex::new(VideoList::new())),
        };

        feed_view.listen_backend_messages(backend);
        feed_view
    }

    fn listen_backend_messages(&self, backend: Arc<impl Backend + Send + Sync + 'static>) {
        let loading_id = self.loading_id.clone();
        let actions = self.actions.clone();
        let config = self.config.clone();
        let video_list = self.video_list.clone();
        tokio::spawn(async move {
            {
                let mut loading_id = loading_id.lock();
                *loading_id = Some(actions.start_status(LOADING_STRING));
            }

            let mut receiver = backend.subscribe();
            while let Some(message) = receiver.recv().await {
                Self::handle_backend_message(
                    message,
                    loading_id.clone(),
                    actions.clone(),
                    config.clone(),
                    video_list.clone(),
                )
                .await;
            }
        });
    }

    async fn handle_backend_message(
        message: BackendMessage<Video>,
        loading_id: Arc<Mutex<Option<usize>>>,
        actions: Actions,
        config: Arc<ConfigHandler>,
        video_list: Arc<Mutex<VideoList>>,
    ) {
        match message {
            BackendMessage::Error(error) => actions.handle_error_async(error, true).await,
            BackendMessage::FinishedFetching => {
                let mut loading_id = loading_id.lock();
                if let Some(loading_id) = *loading_id {
                    actions.finish_status(loading_id);
                }
                *loading_id = None;
            }
            _ => {
                {
                    let mut video_list = video_list.lock();
                    video_list
                        .handle_backend_message(message, config.clone().last_played_timestamp());
                }
                actions.redraw_async().await;
            }
        }
    }

    fn set_current_as_last_played(&mut self) {
        let Some(last_played_timestamp) = self.video_list.lock().current_timestamp() else { return };
        self.update_last_played_timestamp(last_played_timestamp);
    }

    fn update_last_played_timestamp(&mut self, last_played_timestamp: i64) {
        {
            self.video_list
                .lock()
                .update_last_played_timestamp(last_played_timestamp);
        }

        let config = self.config.clone();
        let actions = self.actions.clone();
        tokio::spawn(async move {
            actions
                .redraw_or_error_async(
                    config
                        .set_last_played_timestamp(last_played_timestamp)
                        .await,
                    true,
                )
                .await;
        });
    }

    fn play(&mut self) {
        let selected_videos = self.video_list.lock().selected_videos();

        if let Some(newest_video) = selected_videos.first() {
            let finish_status = self.actions.show_label("Playing...");
            self.update_last_played_timestamp(newest_video.date().timestamp());

            let player = self.get_player();
            let actions = self.actions.clone();
            tokio::spawn(async move {
                let videos = selected_videos.iter().map(|video| video.url());
                let play_result = Command::new(player)
                    .args(videos)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .await;

                finish_status();
                actions.redraw_or_error_async(play_result, true).await;
            });
        }
    }

    fn get_player(&self) -> String {
        env::args()
            .skip_while(|arg| arg != "--player")
            .nth(1)
            .unwrap_or_else(|| self.config.player())
    }
}

impl Component for FeedView {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        let description_height = 10;
        let description_y = area.height - description_height;
        let list_area = Rect::new(area.x, 0, area.width, description_y - 1);
        let description_area = Rect::new(area.x, description_y, area.width, description_height);

        let video_list = self.video_list.lock();
        let description = video_list.current_description();

        let metadata_width = 35;
        let title_area = Rect::new(
            list_area.x,
            list_area.y,
            list_area.width - metadata_width - 3,
            list_area.height,
        );
        let metadata_area = Rect::new(
            list_area.x + list_area.width - metadata_width,
            list_area.y + 1,
            metadata_width,
            list_area.height - 1,
        );

        let list = video_list.list(list_area.height.into());
        let styled_list = list
            .block(Block::default().title("Videos"))
            .style(Style::default().fg(Color::White));

        let metadata_list = video_list.metadata_list(list_area.height.into());
        let styled_metadata_list = metadata_list
            .block(Block::default())
            .style(Style::default().fg(Color::White));

        f.render_widget(styled_list, title_area);
        f.render_widget(styled_metadata_list, metadata_area);
        f.render_widget(description, description_area);
    }

    fn handle_event(&mut self, event: Event) {
        if let Event::Key(event) = event {
            match event.code {
                KeyCode::Up => self.video_list.lock().move_up(),
                KeyCode::Down => self.video_list.lock().move_down(),
                KeyCode::Char('j') => self.video_list.lock().move_down(),
                KeyCode::Char('k') => self.video_list.lock().move_up(),
                KeyCode::Char('g') => self.video_list.lock().move_top(),
                KeyCode::Char('G') => self.video_list.lock().move_bottom(),
                KeyCode::Char('a') => self.video_list.lock().deselect_all(),
                KeyCode::Char(' ') => self.video_list.lock().toggle_current(),
                KeyCode::Char('p') => self.play(),
                KeyCode::Char('n') => self.set_current_as_last_played(),
                _ => return,
            }
        }

        self.actions.redraw();
    }

    fn registered_events(&self) -> Vec<(String, String)> {
        vec![
            (String::from("j"), String::from("Down")),
            (String::from("k"), String::from("Up")),
            (String::from("g"), String::from("Top")),
            (String::from("G"), String::from("Bottom")),
            (String::from("Space"), String::from("Select")),
            (String::from("p"), String::from("Play")),
            (String::from("n"), String::from("Update last played")),
            (String::from("a"), String::from("Deselect all")),
        ]
    }
}
