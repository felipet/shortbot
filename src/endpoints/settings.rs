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

//! Handler for the /settings command.

use crate::{
    HandlerResult, ShortBotDialogue, State, endpoints,
    keyboards::*,
    users::{UserHandler, user_lang_code},
};
use std::sync::Arc;
use teloxide::{
    adaptors::Throttle,
    prelude::*,
    types::{MessageId, ParseMode},
};
use tracing::{debug, error};

/// Start handler.
#[tracing::instrument(
    name = "Settings handler",
    skip(bot, msg, dialogue, user_handler),
    fields(
        chat_id = %msg.chat.id,
    )
)]
pub async fn settings(
    bot: Throttle<Bot>,
    msg: Message,
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
    let lang_code = &user_lang_code(&user_id, user_handler.clone(), None).await;

    let msg_id = bot
        .send_message(
            msg.chat.id,
            format!(
                "🎛️ <b><ins>{}</ins></b>",
                if lang_code == "es" {
                    "Menú de ajustes"
                } else {
                    "Settings menu"
                }
            ),
        )
        .parse_mode(ParseMode::Html)
        .disable_notification(true)
        .reply_markup(settings_keyboard(lang_code))
        .await?
        .id;

    // Update the dialogue status
    dialogue.update(State::Settings { msg_id }).await?;

    Ok(())
}

/// Start handler.
#[tracing::instrument(
    name = "Settings callback handler",
    skip(bot, dialogue, query, user_handler, msg_id),
    fields(
        chat_id = %dialogue.chat_id(),
    )
)]
pub async fn settings_callback(
    bot: Throttle<Bot>,
    dialogue: ShortBotDialogue,
    query: CallbackQuery,
    user_handler: Arc<UserHandler>,
    msg_id: MessageId,
) -> HandlerResult {
    let user_id = if let Some(user_id) = dialogue.chat_id().as_user() {
        user_id
    } else {
        error!("Brief handler called by a non-user of Telegram");
        return Ok(());
    };
    let lang_code = &user_lang_code(&user_id, user_handler.clone(), None).await;

    bot.answer_callback_query(query.id).await?;

    let callback_choice = query.data.unwrap();

    match callback_choice.as_str() {
        "subscriptions" => {
            debug!("Moving to subscriptions menu");
            bot.edit_message_text(
                dialogue.chat_id(),
                msg_id,
                format!(
                    "🎛️ <b><ins>{}</ins></b>",
                    if lang_code == "es" {
                        "Gestionar subscripciones"
                    } else {
                        "Subscriptions menu"
                    }
                ),
            )
            .parse_mode(ParseMode::Html)
            .await?;
            endpoints::subscriptions_menu(bot.clone(), dialogue, user_handler.clone()).await?;
        }
        "plan" => {
            check_user_plan(&bot, &dialogue, user_handler, user_id).await?;
        }
        "exit" => {
            bot.delete_message(dialogue.chat_id(), msg_id).await?;
            dialogue.exit().await?
        }
        _ => {
            bot.edit_message_text(dialogue.chat_id(), msg_id, "*Option not implemented*")
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
            dialogue.exit().await?;
        }
    }

    Ok(())
}

async fn check_user_plan(
    bot: &Throttle<Bot>,
    dialogue: &ShortBotDialogue,
    user_handler: Arc<UserHandler>,
    user_id: UserId,
) -> HandlerResult {
    let lang_code = &user_lang_code(&user_id, user_handler.clone(), None).await;
    let access_level = user_handler.access_level(&user_id).await?;

    bot.send_message(dialogue.chat_id(), subscription_plan_msg(lang_code))
        .disable_notification(true)
        .await?;

    bot.send_message(dialogue.chat_id(), format!("{access_level}"))
        .disable_notification(true)
        .await?;

    bot.send_message(dialogue.chat_id(), subscription_extra_msg(lang_code))
        .disable_notification(true)
        .await?;

    dialogue.exit().await?;

    Ok(())
}

fn subscription_plan_msg(lang_code: &str) -> String {
    let msg = match lang_code {
        "es" => "Tu plan de acceso al bot es:",
        _ => "Your current subscription plan is:",
    };

    msg.to_owned()
}

fn subscription_extra_msg(lang_code: &str) -> String {
    let msg = match lang_code {
        "es" => "Para más información acerca de los planes de acceso, usa el comando /planes",
        _ => "If you need more information about subscription plans, please use the command /plans",
    };

    msg.to_owned()
}
