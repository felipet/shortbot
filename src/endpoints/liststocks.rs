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

use crate::{HandlerResult, ShortBotDialogue, ShortCache, State, keyboards::tickers_grid_keyboard};
use std::sync::Arc;
use teloxide::{adaptors::Throttle, prelude::*};
use tracing::{debug, info};

#[tracing::instrument(
    name = "List stocks handler",
    skip(bot, dialogue, msg, short_cache),
    fields(
        chat_id = %msg.chat.id,
    )
)]
pub async fn list_stocks(
    bot: Throttle<Bot>,
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

    let keyboard_markup = tickers_grid_keyboard(&ibex_market);

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
