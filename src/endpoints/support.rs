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

//! Handler for the /support command.

use crate::{
    HandlerResult,
    users::{UserHandler, user_lang_code},
};
use std::sync::Arc;
use teloxide::{adaptors::Throttle, prelude::*, types::ParseMode};
use tracing::error;

/// Support handler.
#[tracing::instrument(
    name = "Support handler",
    skip(bot, msg, user_handler),
    fields(
        chat_id = %msg.chat.id,
    )
)]
pub async fn support(
    bot: Throttle<Bot>,
    msg: Message,
    user_handler: Arc<UserHandler>,
) -> HandlerResult {
    // First, try to retrieve the user of the chat.
    let user_id = match &msg.from {
        Some(user) => user.id,
        None => {
            error!("A non-user of Telegram is attempting to use the bot");
            return Ok(());
        }
    };

    let lang_code = user_lang_code(&user_id, user_handler.clone(), None).await;

    let message = match lang_code.as_str() {
        "es" => _support_es(),
        _ => _support_en(),
    };

    bot.send_message(msg.chat.id, message)
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(())
}

/// Support handler (English version).
fn _support_en() -> String {
    include_str!("../../data/templates/support_en.txt").to_string()
}

/// Support handler (Spanish version).
fn _support_es() -> String {
    include_str!("../../data/templates/support_es.txt").to_string()
}
