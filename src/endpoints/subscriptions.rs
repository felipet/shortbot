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
    HandlerResult, ShortBotDialogue, ShortCache, State,
    keyboards::{companies_keyboard, tickers_grid_keyboard},
    users::{UserConfig, UserHandler},
};
use std::sync::Arc;
use teloxide::{adaptors::Throttle, prelude::*, types::MessageId};
use tracing::{debug, error, info};

#[tracing::instrument(
    name = "Subscriptions menu",
    skip(bot, dialogue, _user_handler),
    fields(
        chat_id = %dialogue.chat_id(),
    )
)]
pub async fn subscriptions_menu(
    bot: Throttle<Bot>,
    dialogue: ShortBotDialogue,
    _user_handler: Arc<UserHandler>,
) -> HandlerResult {
    let msg_id = bot
        .send_message(dialogue.chat_id(), "Feature not implemented!")
        .await?
        .id;

    dialogue.update(State::Subscriptions { msg_id }).await?;

    Ok(())
}

#[tracing::instrument(
    name = "Subscriptions callback",
    skip(_bot, dialogue, _query, _user_handler, _msg_id),
    fields(
        chat_id = %dialogue.chat_id(),
    )
)]
pub async fn subscriptions_callback(
    _bot: Throttle<Bot>,
    dialogue: ShortBotDialogue,
    _query: CallbackQuery,
    _user_handler: Arc<UserHandler>,
    _msg_id: MessageId,
) -> HandlerResult {
    todo!()
}
