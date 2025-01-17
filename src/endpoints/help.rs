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

//! Handler for the /help command.

use crate::{CommandEng, CommandSpa, HandlerResult};
use teloxide::{prelude::*, types::ParseMode, utils::command::BotCommands};
use tracing::{debug, info};

/// Help handler.
#[tracing::instrument(
    name = "Help handler",
    skip(bot, msg),
    fields(
        chat_id = %msg.chat.id,
    )
)]
pub async fn help(bot: Bot, msg: Message) -> HandlerResult {
    info!("Command /help requested");

    // First, try to retrieve the user of the chat.
    let lang_code = match msg.from {
        Some(user) => user.language_code.clone(),
        None => None,
    };

    debug!("The user's language code is: {:?}", lang_code);

    let message = match lang_code {
        Some(lang_code) => match lang_code.as_str() {
            "es" => _help_es(),
            _ => _help_en(),
        },
        _ => _help_en(),
    };

    bot.send_message(msg.chat.id, message)
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(())
}

/// Help handler (English version).
fn _help_en() -> String {
    format!(
        "{}\n\n⚙️{}",
        include_str!("../../data/templates/help_en.txt"),
        CommandEng::descriptions(),
    )
}

/// Help handler (Spanish version).
fn _help_es() -> String {
    format!(
        "{}\n\n⚙️{}",
        include_str!("../../data/templates/help_es.txt"),
        CommandSpa::descriptions(),
    )
}
