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
    CommandEng, CommandSpa, State, WebServerState, configuration::Settings, endpoints, handlers,
    telemetry::configure_tracing, users::UserHandler,
};
use std::{net::SocketAddr, process::exit, str::FromStr, sync::Arc};
use teloxide::{
    adaptors::throttle::Limits, dispatching::dialogue::InMemStorage,
    payloads::SetMyCommandsSetters, prelude::*, requests::RequesterExt, update_listeners::webhooks,
    utils::command::BotCommands,
};
use tokio::net::TcpListener;
use tracing::{debug, error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the settings.
    let settings = Settings::new().expect("Failed to parse configuration files.");

    // Initialize the tracing subsystem.
    configure_tracing(settings.tracing_level.as_str());

    // Initialize the short cache.
    let short_cache = shortbot::ShortCache::connect_backend(&settings.database).await?;

    // Set up the user's metadata DB.
    let user_handler = match UserHandler::new(&settings.users_db).await {
        Ok(uh) => uh,
        Err(e) => {
            error!("An error occurred while attempting to connect to the user's DB:\n{e}");
            exit(69)
        }
    };

    // Instance a throttled bot, to avoid reaching the message limits when broadcast messages are sent.
    let bot = Bot::new(settings.application.api_token.expose_secret()).throttle(Limits::default());

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
        .nest("/adm", main_router)
        .fallback_service(bot_router);

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
            Arc::new(user_handler),
            InMemStorage::<State>::new()
        ])
        .enable_ctrlc_handler()
        .build()
        .dispatch_with_listener(listener, LoggingErrorHandler::with_custom_text("shortbot"))
        .await;

    info!("Gracefully closed ShortBot server");

    Ok(())
}
