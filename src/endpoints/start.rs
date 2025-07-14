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

//! Handler for the /start command.

use crate::HandlerResult;
use teloxide::{adaptors::Throttle, prelude::*};
use tracing::{debug, info};

/// Start handler.
#[tracing::instrument(
    name = "Start handler",
    skip(bot, msg),
    fields(
        chat_id = %msg.chat.id,
    )
)]
pub async fn start(bot: Throttle<Bot>, msg: Message) -> HandlerResult {
    info!("Command /start requested");

    let client_name = get_client_name(&msg);

    // Let's ry to retrieve the user of the chat.
    let lang_code = match msg.from {
        Some(user) => user.language_code.clone(),
        None => None,
    };

    debug!("The user's language code is: {:?}", lang_code);

    let message = match lang_code {
        Some(lang_code) => match lang_code.as_str() {
            "es" => _start_es(&client_name),
            _ => _start_en(&client_name),
        },
        _ => _start_en(&client_name),
    };

    bot.send_message(msg.chat.id, message).await?;

    Ok(())
}

/// Get a human-friendly identifier for the client of the chat.
fn get_client_name(msg: &Message) -> String {
    if let Some(name) = msg.chat.first_name() {
        String::from(name)
    } else {
        match msg.chat.username() {
            Some(username) => String::from(username),
            // When no identifier is accessible for the client, call it "investor".
            None => String::from("investor"),
        }
    }
}

/// Start handler (English version).
fn _start_en(username: &str) -> String {
    format!(
        include_str!("../../data/templates/welcome_en.txt"),
        username,
    )
}

/// Start handler (Spanish version).
fn _start_es(username: &str) -> String {
    format!(
        include_str!("../../data/templates/welcome_es.txt"),
        username,
    )
}
