mod error;
mod config;
mod feed;

use config::ConfigHandler;
use feed::Feed;

#[tokio::main]
async fn main() {
    match ConfigHandler::new().await {
        Ok(mut config_handler) => {
            println!("config: {:?}", config_handler.config);
            // config_handler
            //     .add_subscription(
            //         "https://www.youtube.com/feeds/videos.xml?channel_id=UC4Je3NiGWBA29x0jVeNmA2Q"
            //             .to_string(),
            //     )
            //     .await
            //     .expect("Failed to add subscription");

            match Feed::from_config(config_handler.config).await {
                Ok(feed) => {
                    for video in feed.videos {
                        println!("{}", video.get_label());
                    }
                },
                Err(error) => println!("Error fetching feed: {:?}", error),
            }
        },
        Err(error) => println!("error initializing config: {:?}", error),
    }
}
