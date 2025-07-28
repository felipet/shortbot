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

use crate::{
    CommandEng, CommandSpa, HandlerResult,
    users::{UserHandler, user_lang_code},
};
use std::sync::Arc;
use teloxide::{adaptors::Throttle, prelude::*, types::ParseMode, utils::command::BotCommands};
use tracing::error;

/// Help handler.
#[tracing::instrument(
    name = "Help handler",
    skip(bot, msg, user_handler),
    fields(
        chat_id = %msg.chat.id,
    )
)]
pub async fn help(
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
    let lang_code = &user_lang_code(&user_id, user_handler.clone(), None).await;
    let help_section = help_section(msg.text());

    let help_msg = match help_section {
        "subscription" | "subscriptions" | "subscripciones" | "subscripcion" => {
            subscriptions_help(lang_code)
        }
        _ => main_help(lang_code),
    };

    bot.send_message(msg.chat.id, help_msg)
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(())
}

fn subscriptions_help(lang_code: &str) -> String {
    match lang_code {
        "es" => include_str!("../../data/templates/help_subscriptions_es.txt").to_string(),
        _ => include_str!("../../data/templates/help_subscriptions_en.txt").to_string(),
    }
}

fn main_help(lang_code: &str) -> String {
    match lang_code {
        "es" => _help_es(),
        _ => _help_en(),
    }
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

fn help_section(msg: Option<&str>) -> &str {
    if let Some(msg) = msg {
        match msg.split(" ").last() {
            Some(section) => section.trim_end(),
            None => "main",
        }
    } else {
        "main"
    }
}
