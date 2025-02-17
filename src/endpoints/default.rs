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

//! Handler for the /help command.

use crate::HandlerResult;
use teloxide::{prelude::*, types::ParseMode};
use tracing::{debug, info};

/// Help handler.
#[tracing::instrument(
    name = "Default handler",
    skip(bot, msg),
    fields(
        chat_id = %msg.chat.id,
    )
)]
pub async fn default(bot: Bot, msg: Message) -> HandlerResult {
    info!("Garbage sent");

    // First, try to retrieve the user of the chat.
    let lang_code = match msg.from {
        Some(user) => user.language_code.clone(),
        None => None,
    };

    debug!("The user's language code is: {:?}", lang_code);

    let message = match lang_code.as_deref().unwrap_or("en") {
        "es" => _warning_es(),
        _ => _warning_en(),
    };

    bot.send_message(msg.chat.id, message)
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(())
}

fn _warning_es() -> String {
    include_str!("../../data/templates/warning_es.txt").to_owned()
}

fn _warning_en() -> String {
    include_str!("../../data/templates/warning_en.txt").to_owned()
}
