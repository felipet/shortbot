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

use crate::{
    HandlerResult, ShortBotDialogue, ShortCache, State,
    keyboards::{companies_keyboard, tickers_grid_keyboard},
    users::{UserConfig, UserHandler},
};
use std::sync::Arc;
use teloxide::{adaptors::Throttle, prelude::*, types::MessageId};
use tracing::{debug, error, info};

#[tracing::instrument(
    name = "List stocks handler",
    skip(bot, dialogue, msg, short_cache, user_handler),
    fields(
        chat_id = %msg.chat.id,
    )
)]
pub async fn list_stocks(
    bot: Throttle<Bot>,
    dialogue: ShortBotDialogue,
    msg: Message,
    short_cache: Arc<ShortCache>,
    user_handler: Arc<UserHandler>,
) -> HandlerResult {
    info!("Command /short requested");

    // Let's try to retrieve the user's language.
    let (user_id, lang_code) = match &msg.from {
        Some(user) => (&user.id, user.language_code.clone()),
        None => {
            error!("List stocks called by a non-user of Telegram");
            return Ok(());
        }
    };

    debug!("The user's language code is: {:?}", lang_code);

    let ibex_market = short_cache.ibex35_listing().await?;

    let user_cfg: UserConfig = match user_handler.user_config(user_id).await {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Error while obtaining user's config from the DB: {e}");
            return Ok(());
        }
    };

    let (message, keyboard_markup) = if user_cfg.prefer_tickers {
        debug!("The user prefers tickers");
        (
            _select_ticker_message(lang_code.as_deref()),
            tickers_grid_keyboard(&ibex_market),
        )
    } else {
        debug!("The user prefers company names");
        (
            _select_company_message(lang_code.as_deref()),
            companies_keyboard(&ibex_market, None),
        )
    };

    let msg_id = bot
        .send_message(msg.chat.id, message)
        .reply_markup(keyboard_markup)
        .await?
        .id;

    info!("Stocks listed, moving to State::ReceiveStock");

    if user_cfg.prefer_tickers {
        dialogue.update(State::ReceiveStock { msg_id }).await?;
    } else {
        dialogue.update(State::ListStocksByName { msg_id }).await?;
    }

    Ok(())
}

#[tracing::instrument(
    name = "List stocks by name handler",
    skip(bot, dialogue, short_cache, q, msg_id),
    fields(
        chat_id = %dialogue.chat_id(),
    )
)]
pub async fn list_stock_by_name(
    bot: Throttle<Bot>,
    dialogue: ShortBotDialogue,
    short_cache: Arc<ShortCache>,
    q: CallbackQuery,
    msg_id: MessageId,
) -> HandlerResult {
    bot.delete_message(dialogue.chat_id(), msg_id).await?;
    let starting_char = q.data.unwrap();
    // Let's try to retrieve the user's language.
    let lang_code = q.from.language_code.as_deref();
    debug!("The user's language code is: {:?}", lang_code);

    // Filter out the companies whose name doesn't start by `starting_char`.
    let ibex_market = short_cache.ibex35_listing().await?;

    let keyboard_markup = companies_keyboard(&ibex_market, Some(&starting_char));

    let msg_id = bot
        .send_message(dialogue.chat_id(), _select_company_message(lang_code))
        .reply_markup(keyboard_markup)
        .await?
        .id;

    dialogue.update(State::ReceiveStock { msg_id }).await?;

    Ok(())
}

fn _select_ticker_message(lang_code: Option<&str>) -> String {
    let lang_code = lang_code.unwrap_or("en");

    match lang_code {
        "es" => String::from("Selecciona un ticker:"),
        _ => String::from("Select a ticker:"),
    }
}

fn _select_company_message(lang_code: Option<&str>) -> String {
    let lang_code = lang_code.unwrap_or("en");

    match lang_code {
        "es" => String::from("Selecciona la letra por la que empieza el nombre de la empresa:"),
        _ => String::from("Choose the starting letter for the company's name:"),
    }
}
