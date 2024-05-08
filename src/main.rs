//! Main file of the Shortbot

use secrecy::ExposeSecret;
use shortbot::telemetry::{get_subscriber, init_subscriber};
use shortbot::{configuration::Settings, handlers, State};
use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::prelude::*;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the settings.
    let settings = Settings::new().expect("Failed to parse configuration files.");

    // Initialize the tracing subsystem.
    let subscriber = get_subscriber(settings.tracing_level.as_str());
    init_subscriber(subscriber);

    info!("Started ShortBot server");

    let bot = Bot::new(settings.application.api_token.expose_secret());

    info!("Dispatching");

    Dispatcher::builder(bot, handlers::schema())
        .dependencies(dptree::deps![InMemStorage::<State>::new()])
        .build()
        .dispatch()
        .await;

    info!("Closed ShortBot server");

    Ok(())
}
