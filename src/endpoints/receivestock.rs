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

//! Handler that lists all the available stocks to the client.

use crate::finance::AliveShortPositions;
use crate::finance::CNMVProvider;
use crate::finance::Ibex35Market;
use crate::{HandlerResult, ShortBotDialogue};
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use tracing::{debug, info};

#[tracing::instrument(
    name = "Receive stock handler",
    skip(bot, dialogue, stock_market, q),
    fields(
        chat_id = %dialogue.chat_id(),
    )
)]
pub async fn receive_stock(
    bot: Bot,
    dialogue: ShortBotDialogue,
    stock_market: Arc<Ibex35Market>,
    q: CallbackQuery,
) -> HandlerResult {
    // Let's try to retrieve the user of the chat.
    let lang_code = match q.from.language_code.as_deref().unwrap_or("en") {
        "es" => "es",
        _ => "en",
    };

    debug!("The user's language code is: {:?}", lang_code);

    if let Some(ticker) = &q.data {
        let message = match lang_code {
            "es" => _chose_es(stock_market.stock_by_ticker(ticker).unwrap().name()),
            _ => _chose_en(stock_market.stock_by_ticker(ticker).unwrap().name()),
        };

        bot.send_message(dialogue.chat_id(), message)
            .parse_mode(ParseMode::Html)
            .await?;
        info!("Selected stock: {}", ticker);
    } else {
        bot.send_message(
            dialogue.chat_id(),
            if lang_code == "es" {
                "Ninguna empresa seleccionada."
            } else {
                "No stock was given."
            },
        )
        .await?;
        info!("No valid ticker was received");
        info!("Short position request served");
        dialogue.exit().await?;
        return Ok(());
    }

    let provider = CNMVProvider::new();
    let stock_object = stock_market.stock_by_ticker(&q.data.unwrap()[..]).unwrap();
    debug!("Stock descriptor: {stock_object}");
    let positions = provider.short_positions(stock_object).await;
    debug!("Received AliveShortPositions: {:?}", positions);

    if positions.is_ok() {
        let shorts = positions.unwrap();

        if shorts.total <= 0.0 {
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
