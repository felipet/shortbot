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

//! Handler that lists all the available stocks to the client.

use crate::{HandlerResult, ShortBotDialogue, ShortCache};
use data_harvest::domain::AliveShortPositions;
use std::sync::Arc;
use teloxide::{
    adaptors::Throttle,
    prelude::*,
    types::{MessageId, ParseMode},
};
use tracing::{debug, info};

#[tracing::instrument(
    name = "Receive stock handler",
    skip(bot, dialogue, short_cache, q),
    fields(
        chat_id = %dialogue.chat_id(),
    )
)]
pub async fn receive_stock(
    bot: Throttle<Bot>,
    dialogue: ShortBotDialogue,
    short_cache: Arc<ShortCache>,
    q: CallbackQuery,
    msg_id: MessageId,
) -> HandlerResult {
    let payload = &q.data.unwrap();

    // Let's try to retrieve the user of the chat.
    let lang_code = match q.from.language_code.as_deref().unwrap_or("en") {
        "es" => "es",
        _ => "en",
    };
    debug!("The user's language code is: {:?}", lang_code);

    // Delete the previous keyboard and display a message that contains the name of the chosen ticker/company.
    bot.delete_message(dialogue.chat_id(), msg_id).await?;
    let message = match lang_code {
        "es" => _chose_es(payload),
        _ => _chose_en(payload),
    };
    bot.send_message(dialogue.chat_id(), message)
        .parse_mode(ParseMode::Html)
        .await?;

    let positions = short_cache.short_position(payload).await;
    debug!("Received AliveShortPositions: {:?}", positions);

    if positions.is_ok() {
        let shorts = positions.unwrap();

        if shorts.positions.is_empty() {
            bot.send_message(dialogue.chat_id(), _no_shorts_msg(lang_code))
                .parse_mode(ParseMode::Html)
                .await?;
        } else {
            // Build the second part of the message only if there are alive short positions.
            let message = match lang_code {
                "es" => _shorts_msg_es(&shorts),
                _ => _shorts_msg_en(&shorts),
            };
            bot.send_message(dialogue.chat_id(), message)
                .parse_mode(ParseMode::Html)
                .await?;
        }
    } else {
        let message = if lang_code == "es" {
            "Información no disponible"
        } else {
            "Information not available"
        };
        bot.send_message(dialogue.chat_id(), message).await?;
    }

    info!("Short position request served");
    dialogue.exit().await?;

    Ok(())
}

fn _chose_es(stock_name: &str) -> String {
    format!(
        include_str!("../../data/templates/chose_es.txt"),
        stock_name,
    )
}

fn _chose_en(stock_name: &str) -> String {
    format!(
        include_str!("../../data/templates/chose_en.txt"),
        stock_name,
    )
}

fn _no_shorts_msg(lang_code: &str) -> &str {
    match lang_code {
        "es" => "<b>No hay posiciones en corto notificadas</b> (>=0.5%)",
        _ => "<b>There are no open short positions</b> (>= 0.5%)",
    }
}

fn _shorts_msg_en(shorts: &AliveShortPositions) -> String {
    let s = format!(
        include_str!("../../data/templates/short_position_en.txt"),
        shorts.total,
    );
    format!("{}{}{}", s, "\n\nList of individual positions:\n", shorts,)
}

fn _shorts_msg_es(shorts: &AliveShortPositions) -> String {
    let s = format!(
        include_str!("../../data/templates/short_position_es.txt"),
        shorts.total,
    );
    format!(
        "{}{}{}",
        s, "\n\nLista de posiciones individuales:\n", shorts,
    )
}
