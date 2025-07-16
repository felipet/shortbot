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

use crate::{HandlerResult, ShortBotDialogue, State, users::UserHandler};
use std::sync::Arc;
use teloxide::{
    adaptors::Throttle,
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, MessageId, ParseMode},
};
use tracing::{debug, error};

/// Start handler.
#[tracing::instrument(
    name = "Settings handler",
    skip(bot, msg, dialogue),
    fields(
        chat_id = %msg.chat.id,
    )
)]
pub async fn settings(
    bot: Throttle<Bot>,
    msg: Message,
    dialogue: ShortBotDialogue,
) -> HandlerResult {
    let keyboard = InlineKeyboardMarkup::default()
        .append_row(vec![InlineKeyboardButton::callback(
            "📺 Display settings".to_string(),
            "display_main",
        )])
        .append_row(vec![InlineKeyboardButton::callback(
            "📰 My subscriptions".to_string(),
            "subscriptions",
        )])
        .append_row(vec![InlineKeyboardButton::callback(
            "🧾 My plan".to_string(),
            "plan",
        )]);

    let msg_id = bot
        .send_message(msg.chat.id, "🎛️ <b><ins>Settings menu</ins></b>")
        .parse_mode(teloxide::types::ParseMode::Html)
        .disable_notification(true)
        .reply_markup(keyboard)
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
    //let user = match &msg.from {
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

    bot.answer_callback_query(query.id).await?;

    let callback_choice = query.data.unwrap();

    match callback_choice.as_str() {
        "subscriptions" => {
            check_user_subscriptions(&bot, &dialogue, user_handler, user_id, msg_id).await?;
        }
        "plan" => {
            check_user_plan(&bot, &dialogue, user_handler, user_id).await?;
        }
        "exit" => {
            bot.delete_message(dialogue.chat_id(), msg_id).await?;
            dialogue.exit().await?
        }
        _ => {
            bot.send_message(dialogue.chat_id(), "*Option not implemented*")
                .disable_notification(true)
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
            dialogue.exit().await?;
        }
    }

    Ok(())
}

async fn check_user_subscriptions(
    bot: &Throttle<Bot>,
    dialogue: &ShortBotDialogue,
    user_handler: Arc<UserHandler>,
    user_id: UserId,
    msg_id: MessageId,
) -> HandlerResult {
    let subscriptions = user_handler.subscriptions(&user_id).await?;

    if let Some(subs) = subscriptions {
        bot.send_message(
            dialogue.chat_id(),
            "💹 These are your current subscriptions:",
        )
        .disable_notification(true)
        .await?;

        bot.send_message(dialogue.chat_id(), format!("{subs}"))
            .disable_notification(true)
            .await?;

        let keyboard = InlineKeyboardMarkup::default()
            .append_row(vec![InlineKeyboardButton::callback(
                "➕ Add new subscriptions".to_string(),
                "add_subscriptions",
            )])
            .append_row(vec![InlineKeyboardButton::callback(
                "➖ Delete a subscription".to_string(),
                "delete_subscriptions",
            )])
            .append_row(vec![InlineKeyboardButton::callback(
                "✖️ Clear my subscriptions".to_string(),
                "clear_subscriptions",
            )])
            .append_row(vec![InlineKeyboardButton::callback(
                "🏃‍♀️‍➡️ Exit".to_string(),
                "exit",
            )]);

        bot.edit_message_reply_markup(dialogue.chat_id(), msg_id)
            .reply_markup(keyboard)
            .await?;
    } else {
        bot.send_message(
            dialogue.chat_id(),
            "You don't have any subscriptions at this moment",
        )
        .disable_notification(true)
        .await?;

        dialogue.exit().await?;
    }

    Ok(())
}

async fn check_user_plan(
    bot: &Throttle<Bot>,
    dialogue: &ShortBotDialogue,
    user_handler: Arc<UserHandler>,
    user_id: UserId,
) -> HandlerResult {
    let access_level = user_handler.access_level(&user_id).await?;

    bot.send_message(dialogue.chat_id(), "Your current subscription plan is:")
        .disable_notification(true)
        .await?;

    bot.send_message(dialogue.chat_id(), format!("{access_level}"))
        .disable_notification(true)
        .await?;

    bot.send_message(
        dialogue.chat_id(),
        "If you need more information about subscription plans, please use the command /plans",
    )
    .disable_notification(true)
    .await?;

    dialogue.exit().await?;

    Ok(())
}
