// Copyright 2024 Felipe Torres González
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
use shortbot::finance::load_ibex35_companies;
use shortbot::{
    configuration::Settings,
    handlers,
    telemetry::{get_subscriber, init_subscriber},
    State, IBEX35_STOCK_DESCRIPTORS,
};
use shortbot::{CommandEng, CommandSpa};
use std::sync::Arc;
use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::payloads::SetMyCommandsSetters;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use tracing::{debug, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the settings.
    let settings = Settings::new().expect("Failed to parse configuration files.");

    // Initialize the tracing subsystem.
    let subscriber = get_subscriber(settings.tracing_level.as_str());
    init_subscriber(subscriber);

    let ibexdata_path = std::path::PathBuf::from(settings.data_path).join(IBEX35_STOCK_DESCRIPTORS);

    let ibex35 = load_ibex35_companies(ibexdata_path.as_os_str().to_str().unwrap())
        .expect("Failed to parse IBEX35 companies.");
    let ibex35 = Arc::new(ibex35);

    info!("Started ShortBot server");

    let bot = Bot::new(settings.application.api_token.expose_secret());

    // Configure the supported languages of the Bot.
    debug!("Setting up commands of the bot");
    bot.set_my_commands(CommandSpa::bot_commands())
        .language_code("es")
        .await?;
    bot.set_my_commands(CommandEng::bot_commands())
        .language_code("en")
        .await?;

    info!("Dispatching");

    let ibex35_clone = Arc::clone(&ibex35);

    Dispatcher::builder(bot, handlers::schema())
        .dependencies(dptree::deps![ibex35_clone, InMemStorage::<State>::new()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    info!("Gracefully closed ShortBot server");

    Ok(())
}
