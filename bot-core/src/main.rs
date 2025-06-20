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

use bot_core::{CommandEng, CommandSpa, clientlib_startup};
use bot_core::{
    State, handlers,
    telemetry::{get_subscriber, init_subscriber},
};

use configuration::Settings;
use secrecy::ExposeSecret;
use std::{net::SocketAddr, str::FromStr, sync::Arc};
use teloxide::{
    dispatching::dialogue::InMemStorage, payloads::SetMyCommandsSetters, prelude::*,
    update_listeners::webhooks, utils::command::BotCommands,
};
use tokio::{net::TcpListener, signal};
use tracing::{debug, error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the settings.
    let settings = Settings::new().expect("Failed to parse configuration files.");

    // Initialize the tracing subsystem.
    let subscriber = get_subscriber(settings.tracing_level.as_str());
    init_subscriber(subscriber);

    // Initialize the short cache.
    let short_cache = bot_core::ShortCache::connect_backend(&settings.database).await?;

    let (tx, rx) = tokio::sync::mpsc::channel(10);
    let (mut cache_handler, client_handler) =
        clientlib_startup(&settings, (tx.clone(), rx)).await?;

    let _ = tokio::spawn(async move {
        let _ = cache_handler.start().await;
        info!("Cache handler closed");
    });

    let tx_close = tx.clone();

    tokio::spawn(async move {
        match signal::ctrl_c().await {
            Ok(()) => {
                info!("Signal ctrl-c received!");
                let _ = tx_close.send("stop".to_owned()).await;
                info!("Stop signal sent to the cache handler");
            }
            Err(err) => {
                error!("Unable to listen for shutdown signal: {}", err);
            }
        }
    });

    // Build an Axum HTTP server.
    let main_router: axum::Router<()> =
        axum::Router::new().route("/adm", axum::routing::get(|| async { "Hello, World!" }));

    let http_server_address = SocketAddr::from_str(&format!(
        "{}:{}",
        &settings.application.http_server_host, settings.application.http_server_port
    ))
    .expect("Failed to build a socket using the configuration");

    let tcp_listener = TcpListener::bind(http_server_address)
        .await
        .expect("Failed to bind to the provided address");

    info!("Started ShortBot server");

    let bot = Bot::new(settings.application.api_token.expose_secret());

    // Build a listener based on the axum server.
    let (listener, stop_future, bot_router) = webhooks::axum_to_router(
        bot.clone(),
        webhooks::Options::new(
            http_server_address,
            format!(
                "{}{}",
                settings.application.webhook_url, settings.application.webhook_path
            )
            .parse()
            .unwrap(),
        ),
    )
    .await?;

    // Launch the Axum server.
    let app = axum::Router::new()
        .nest_service("/", bot_router)
        .nest("/bot", main_router);

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
            Arc::new(short_cache),
            Arc::new(client_handler),
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
