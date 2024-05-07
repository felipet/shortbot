//! Main file of the Shortbot

use secrecy::ExposeSecret;
use shortbot::{configuration::Settings, handlers, State};
use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = Settings::new().expect("Failed to parse configuration files.");

    let bot = Bot::new(settings.application.api_token.expose_secret());

    Dispatcher::builder(bot, handlers::schema())
        .dependencies(dptree::deps![InMemStorage::<State>::new()])
        .build()
        .dispatch()
        .await;

    Ok(())
}
