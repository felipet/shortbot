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
    HandlerResult,
    users::{UserHandler, user_lang_code},
};
use std::sync::Arc;
use teloxide::{adaptors::Throttle, prelude::*, types::ParseMode};
use tracing::error;

#[tracing::instrument(
    name = "Plans handler",
    skip(bot, msg, user_handler),
    fields(
        chat_id = %msg.chat.id,
    )
)]
pub async fn plans(
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

    bot.send_message(user_id, _plans_message(lang_code))
        .parse_mode(ParseMode::Html)
        .disable_notification(true)
        .await?;

    Ok(())
}

fn _plans_message(lang_code: &str) -> String {
    match lang_code {
        "es" => "<b>🆓 ¡¡Todas las funconalidades del bot son gratis!!</b> Disfrútalas, y si te apetece colaborar con el proyecto, usa /apoyo para ver como podrías.".into(),
        _ => "<b>🆓 All the bot's features are gratis!! Enjoy, and if you love to use this bot, consider supporting its development, check /support to see how.</b>".into(),
    }
}
