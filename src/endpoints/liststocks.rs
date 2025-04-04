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

use crate::{HandlerResult, ShortBotDialogue, ShortCache, State};
use finance_api::Company;
use std::sync::Arc;
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
};
use tracing::{debug, info};

#[tracing::instrument(
    name = "List stocks handler",
    skip(bot, dialogue, msg, short_cache),
    fields(
        chat_id = %msg.chat.id,
    )
)]
pub async fn list_stocks(
    bot: Bot,
    dialogue: ShortBotDialogue,
    msg: Message,
    short_cache: Arc<ShortCache>,
) -> HandlerResult {
    info!("Command /short requested");

    // Let's try to retrieve the user's language.
    let lang_code = match msg.from {
        Some(user) => user.language_code.clone(),
        None => None,
    };

    debug!("The user's language code is: {:?}", lang_code);

    let ibex_market = short_cache.ibex35_listing().await?;
    let market: Vec<&str> = ibex_market.iter().map(|e| e.ticker()).collect();

    // Present the tickers in a table with 5 columns to reduce the number of rows.
    let cols_per_row: usize = 5;
    let stock_len = market.len();

    // Populate the first row
    let mut keyboard_markup = InlineKeyboardMarkup::new([vec![InlineKeyboardButton::callback::<
        &str,
        &str,
    >(market[0], market[0])]]);

    for company in market.iter().take(cols_per_row).skip(1) {
        keyboard_markup = keyboard_markup.append_to_row(
            0,
            InlineKeyboardButton::callback::<&str, &str>(company, company),
        );
    }

    // Populate rows by chunks of `cols_per_row` buttons
    for i in 1..(stock_len / cols_per_row) {
        for j in 0..cols_per_row {
            keyboard_markup = keyboard_markup.append_to_row(
                i,
                InlineKeyboardButton::callback::<&str, &str>(
                    market[j + i * cols_per_row],
                    market[j + i * cols_per_row],
                ),
            );
        }
    }

    // Finally, add the remainder in case the number of items is not divisible by `cols_per_row`
    if stock_len % cols_per_row != 0 {
        let mut i = stock_len - cols_per_row;
        while i < stock_len {
            keyboard_markup = keyboard_markup.append_to_row(
                stock_len / cols_per_row + 1,
                InlineKeyboardButton::callback::<&str, &str>(market[i], market[i]),
            );

            i += 1;
        }
    }

    bot.send_message(msg.chat.id, _select_stock_message(lang_code.as_deref()))
        .reply_markup(keyboard_markup)
        .await?;

    info!("Stocks listed, moving to State::ReceiveStock");

    dialogue.update(State::ReceiveStock).await?;

    Ok(())
}

fn _select_stock_message(lang_code: Option<&str>) -> String {
    let lang_code = lang_code.unwrap_or("en");

    match lang_code {
        "es" => String::from("Selecciona un ticker:"),
        _ => String::from("Select a ticker:"),
    }
}
