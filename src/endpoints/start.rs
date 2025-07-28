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

use crate::{
    HandlerResult,
    users::{UserHandler, register_new_user, user_lang_code},
};
use std::sync::Arc;
use teloxide::{adaptors::Throttle, prelude::*};
use tracing::{error, info};

/// Start handler.
///
/// # Description
///
/// This handler is usually called once when a new user enters a chat with the Bot. For that reason, the following
/// actions are performed:
///
/// - Try to detect the user's preferred language.
/// - Register the user as an user of the Bot.
/// - Set the language preferences.
#[tracing::instrument(
    name = "Start handler",
    skip(bot, msg, user_handler),
    fields(
        chat_id = %msg.chat.id,
    )
)]
pub async fn start(
    bot: Throttle<Bot>,
    msg: Message,
    user_handler: Arc<UserHandler>,
) -> HandlerResult {
    let client_name = get_client_name(&msg);

    // Let's ry to retrieve the user of the chat.
    let (lang_code, user_id) = match &msg.from {
        Some(user) => (user.language_code.clone(), user.id),
        None => {
            error!("A non-user of Telegram is attempting to use the bot");
            return Ok(());
        }
    };

    match register_new_user(user_id, user_handler.clone(), lang_code.as_deref()).await {
        Ok(_) => info!("A new user started to use the Bot"),
        Err(e) => error!("Error found while attempting to register a new user: {e}"),
    }

    let lang_code = user_lang_code(&user_id, user_handler.clone(), None).await;

    bot.send_message(
        msg.chat.id,
        if lang_code == "es" {
            _start_es(&client_name)
        } else {
            _start_en(&client_name)
        },
    )
    .await?;

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
