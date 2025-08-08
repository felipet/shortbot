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

use crate::{
    HandlerResult, ShortBotDialogue, ShortCache, error_message,
    users::{UserHandler, user_lang_code},
};
use data_harvest::domain::AliveShortPositions;
use std::sync::Arc;
use teloxide::{
    adaptors::Throttle,
    prelude::*,
    types::{MessageId, ParseMode},
};
use tracing::{debug, error, info};

#[tracing::instrument(
    name = "Receive stock handler",
    skip(bot, dialogue, short_cache, user_handler, q, msg_id),
    fields(
        chat_id = %dialogue.chat_id(),
        ticker = %q.data.as_ref().unwrap(),
    )
)]
pub async fn receive_stock(
    bot: Throttle<Bot>,
    dialogue: ShortBotDialogue,
    short_cache: Arc<ShortCache>,
    user_handler: Arc<UserHandler>,
    q: CallbackQuery,
    msg_id: MessageId,
) -> HandlerResult {
    // Delete the previous keyboard and display a message that contains the name of the chosen ticker/company.
    bot.delete_message(dialogue.chat_id(), msg_id).await?;

    let ticker = &q.data.unwrap();

    let user_id = match dialogue.chat_id().as_user() {
        Some(id) => {
            debug!("User {} entered in the settings menu", id);
            id
        }
        None => {
            error!("Settings menu called by a non-user of Telegram");
            return Ok(());
        }
    };
    let lang_code = &user_lang_code(&user_id, user_handler.clone(), None).await;

    match short_report(&bot, dialogue.chat_id(), short_cache, lang_code, ticker).await {
        Ok(_) => info!("Short positions successfully reported"),
        Err(e) => {
            error!("Error found while accessing the stock DB: {e}");
            bot.send_message(dialogue.chat_id(), error_message(lang_code))
                .await?;
            return Err(e);
        }
    }

    dialogue.exit().await?;

    Ok(())
}

/// Function that provides a report of the short positions against a given ticker.
pub(crate) async fn short_report(
    bot: &Throttle<Bot>,
    chat_id: ChatId,
    short_cache: Arc<ShortCache>,
    lang_code: &str,
    ticker: &str,
) -> HandlerResult {
    let message = match lang_code {
        "es" => _chose_es(ticker),
        _ => _chose_en(ticker),
    };

    bot.send_message(chat_id, message)
        .parse_mode(ParseMode::Html)
        .await?;

    let positions = short_cache.short_position(ticker).await;
    debug!("Received AliveShortPositions: {:?}", positions);

    if let Ok(shorts) = positions {
        if shorts.positions.is_empty() {
            bot.send_message(chat_id, _no_shorts_msg(lang_code))
                .parse_mode(ParseMode::Html)
                .await?;
        } else {
            // Build the second part of the message only if there are alive short positions.
            let message = match lang_code {
                "es" => _shorts_msg_es(&shorts),
                _ => _shorts_msg_en(&shorts),
            };
            bot.send_message(chat_id, message)
                .parse_mode(ParseMode::Html)
                .await?;
        }
    } else {
        let message = if lang_code == "es" {
            "Información no disponible"
        } else {
            "Information not available"
        };
        bot.send_message(chat_id, message).await?;
    }

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
