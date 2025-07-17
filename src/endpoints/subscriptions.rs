// Copyright 2025 Felipe Torres González
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
    HandlerResult, ShortBotDialogue, State,
    keyboards::subscriptions_keyboard,
    users::{UserConfig, UserHandler, user_lang_code},
};
use std::sync::Arc;
use teloxide::{
    adaptors::Throttle,
    prelude::*,
    types::{MessageId, ParseMode},
};
use tracing::{debug, error, info};

#[tracing::instrument(
    name = "Subscriptions menu",
    skip(bot, dialogue, user_handler),
    fields(
        chat_id = %dialogue.chat_id(),
    )
)]
pub async fn subscriptions_menu(
    bot: Throttle<Bot>,
    dialogue: ShortBotDialogue,
    user_handler: Arc<UserHandler>,
) -> HandlerResult {
    let user_id = match dialogue.chat_id().as_user() {
        Some(user_id) => user_id,
        None => {
            error!("Subscriptions command called by a non-user");
            return Ok(());
        }
    };
    let lang_code = user_lang_code(&user_id, user_handler.clone(), None).await;

    show_subscriptions(&bot, &dialogue, user_handler.clone(), user_id, true).await?;

    let msg_id = bot
        .send_message(
            dialogue.chat_id(),
            if lang_code == "es" {
                "🗃️ <b>Selecciona una opción:</b>"
            } else {
                "🗃️ <b>Select a following action:</b>"
            },
        )
        .reply_markup(subscriptions_keyboard(&lang_code))
        .parse_mode(ParseMode::Html)
        .await?
        .id;

    dialogue
        .update(State::Subscriptions {
            msg_id: Some(msg_id),
        })
        .await?;

    Ok(())
}

pub(crate) async fn show_subscriptions(
    bot: &Throttle<Bot>,
    dialogue: &ShortBotDialogue,
    user_handler: Arc<UserHandler>,
    user_id: UserId,
    _extended_info: bool,
) -> HandlerResult {
    let lang_code = user_lang_code(&user_id, user_handler.clone(), None).await;

    if let Some(subscriptions) = user_handler.subscriptions(&user_id).await? {
        bot.send_message(
            dialogue.chat_id(),
            if lang_code == "es" {
                "💹 <b>Estas son tus subscripciones:</b>"
            } else {
                "💹 <b>These are your current subscriptions:</b>"
            },
        )
        .parse_mode(ParseMode::Html)
        .await?;
        bot.send_message(dialogue.chat_id(), format!("{subscriptions}"))
            .disable_notification(true)
            .await?;
    } else {
        bot.send_message(
            dialogue.chat_id(),
            if lang_code == "es" {
                "❌ No tienes ninguna subscripción en este momento."
            } else {
                "❌ You don't have any subscriptions at this moment"
            },
        )
        .disable_notification(true)
        .await?;
    }

    Ok(())
}

#[tracing::instrument(
    name = "Subscriptions callback",
    skip(bot, dialogue, query, user_handler, msg_id),
    fields(
        chat_id = %dialogue.chat_id(),
    )
)]
pub async fn subscriptions_callback(
    bot: Throttle<Bot>,
    dialogue: ShortBotDialogue,
    query: CallbackQuery,
    user_handler: Arc<UserHandler>,
    msg_id: Option<MessageId>,
) -> HandlerResult {
    debug!("Subscriptions callback");

    Ok(())
}
