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
use shortbot::prelude::*;
use shortbot::webserver::{setup_bot_router, setup_webserver};
use shortbot::{
    configuration::Settings, handlers, telemetry::configure_tracing, users::UserHandler,
};
use std::{process::exit, sync::Arc};
use teloxide::{
    adaptors::throttle::Limits, dispatching::dialogue::InMemStorage,
    payloads::SetMyCommandsSetters, prelude::*, requests::RequesterExt,
    utils::command::BotCommands,
};
use tracing::{debug, error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the settings.
    let settings = Settings::new().expect("Failed to parse configuration files.");

    // Initialize the tracing subsystem.
    configure_tracing(settings.tracing_level.as_str());
    // Initialize the metrics exporter.
    let (metrics_handle, metrics_upkeep_task) = match setup_metrics() {
        Ok(handle) => {
            let upkeep_task = spawn_metrics_upkeep_task(handle.clone());
            debug!("Metrics upkeep task spawned");
            (handle, upkeep_task)
        }
        Err(e) => {
            error!("Failed to setup metrics: {e}");
            exit(70)
        }
    };

    // Initialize the short cache.
    let short_cache = Arc::new(shortbot::ShortCache::connect_backend(&settings.database).await?);

    // Set up the user's metadata DB.
    let user_handler = match UserHandler::new(&settings.users_db).await {
        Ok(uh) => Arc::new(uh),
        Err(e) => {
            error!("An error occurred while attempting to connect to the user's DB:\n{e}");
            exit(69)
        }
    };

    // Instance a throttled bot, to avoid reaching the message limits when broadcast messages are sent.
    let bot = Bot::new(settings.application.api_token.expose_secret()).throttle(Limits::default());

    let (listener, stop_future, tcp_listener, bot_router) =
        setup_bot_router(bot.clone(), &settings).await?;

    let (app, update_buffer_rx) = setup_webserver(
        user_handler.clone(),
        bot.clone(),
        &settings.application.webhook_token,
        bot_router,
        metrics_handle,
    )?;

    // Updates thread
    handlers::update_handler(
        bot.clone(),
        user_handler.clone(),
        short_cache.clone(),
        update_buffer_rx,
    )
    .await?;

    info!("Started ShortBot server");

    tokio::task::spawn(async move {
        axum::serve(tcp_listener, app)
            .with_graceful_shutdown(stop_future)
            .await
    });
    debug!("Axum server started");

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
            short_cache,
            user_handler,
            InMemStorage::<State>::new()
        ])
        .enable_ctrlc_handler()
        .build()
        .dispatch_with_listener(listener, LoggingErrorHandler::with_custom_text("shortbot"))
        .await;

    metrics_upkeep_task.abort();
    info!("Gracefully closed ShortBot server");

    Ok(())
}
