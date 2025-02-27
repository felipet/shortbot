// Copyright 2024-2025 Felipe Torres González
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
//    You may obtain a copy of the License at
//
//        http://www.apache.org/licenses/LICENSE-2.0
//
//    Unless required by applicable law or agreed to in writing, software
//    distributed under the License is distributed on an "AS IS" BASIS,
//    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//    See the License for the specific language governing permissions and
//    limitations under the License.

//! Main file of the Shortbot

use secrecy::ExposeSecret;
use shortbot::{
    configuration::Settings,
    handlers,
    telemetry::{get_subscriber, init_subscriber},
    State,
};
use shortbot::{CommandEng, CommandSpa};
use std::{net::SocketAddr, str::FromStr, sync::Arc};
use teloxide::{
    dispatching::dialogue::InMemStorage, payloads::SetMyCommandsSetters, prelude::*,
    update_listeners::webhooks, utils::command::BotCommands,
};
use tracing::{debug, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the settings.
    let settings = Settings::new().expect("Failed to parse configuration files.");

    // Initialize the tracing subsystem.
    let subscriber = get_subscriber(settings.tracing_level.as_str());
    init_subscriber(subscriber);

    // Initialize the short cache.
    let short_cache = shortbot::ShortCache::connect_backend(&settings.database).await?;

    // Build an Axum HTTP server.
    //TODO

    let http_server_address = SocketAddr::from_str(&format!(
        "{}:{}",
        &settings.application.http_server_host, settings.application.http_server_port
    ))
    .expect("Failed to build a socket usig the configuration");

    info!("Started ShortBot server");

    let bot = Bot::new(settings.application.api_token.expose_secret());

    // Build a listener based on the axum server.
    let listener = webhooks::axum(
        bot.clone(),
        webhooks::Options::new(
            http_server_address,
            settings.application.webhook_url.parse().unwrap(),
        ),
    )
    .await?;

    // Configure the supported languages of the Bot.
    debug!("Setting up commands of the bot");
    bot.set_my_commands(CommandSpa::bot_commands())
        .language_code("es")
        .await?;
    bot.set_my_commands(CommandEng::bot_commands())
        .language_code("en")
        .await?;

    info!("Dispatching");

    Dispatcher::builder(bot, handlers::schema())
        .dependencies(dptree::deps![
            Arc::new(short_cache),
            InMemStorage::<State>::new()
        ])
        .enable_ctrlc_handler()
        .build()
        .dispatch_with_listener(
            listener,
            LoggingErrorHandler::with_custom_text("Teloxide-Log"),
        )
        .await;

    info!("Gracefully closed ShortBot server");

    Ok(())
}
