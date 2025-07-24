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

use crate::users::user_lang_code;
use crate::{HandlerResult, ShortBotDialogue, UserHandler};
use std::sync::Arc;
use teloxide::{
    adaptors::Throttle,
    prelude::*,
    types::{ParseMode, UserId},
};
use tracing::{debug, info};

/// Function that lists the subscriptions of a bot user.
///
/// # Description
///
/// This function retrieves the current subscriptions of a registered user and sends a list of tickers to the user
/// as bot messages.
///
/// This function is translated in Spanish and English.
///
/// ## Preconditions
///
/// The user identified by `user_id` must be registered in the user's DB.
pub(crate) async fn list_subscriptions(
    bot: Throttle<Bot>,
    dialogue: &ShortBotDialogue,
    user_handler: Arc<UserHandler>,
    user_id: UserId,
) -> HandlerResult {
    let lang_code = user_lang_code(&user_id, user_handler.clone(), None).await;

    if let Some(subscriptions) = user_handler.subscriptions(&user_id).await? {
        info!("Listing user's subscriptions");
        debug!("Subscriptions: {:?}", subscriptions);
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
        debug!("The user has no subscriptions. Nothing to list.");
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
