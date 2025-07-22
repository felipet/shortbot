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
    keyboards::{small_buttons_grid_keyboard, subscriptions_keyboard, tickers_grid_keyboard},
    users::{Subscriptions, UserConfig, UserHandler, user_lang_code},
};
use std::sync::Arc;
use teloxide::{
    adaptors::Throttle,
    prelude::*,
    types::{MessageId, ParseMode},
};
use tracing::{debug, error, info};

/// Main subscriptions handler
///
/// # Description
///
/// This handler is the main entry point to handle user's subscriptions. It shows the current subscriptions of the
/// user, and sends a keyboard that offers adding, deleting or clearing subscriptions.
/// The state machine moves to `State::Subscriptions` which is used to trigger the callback handler that will take
/// the choice from the user.
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

/// Main subscriptions callback handler
///
/// # Description
///
/// This callback handles two stages of the subscriptions menu:
/// 1. The first stage in which the user selects between add, delete or clear subscriptions.
/// 2. The second stage in which the user provides a ticker for the previously selected function.
#[tracing::instrument(
    name = "Subscriptions callback",
    skip(bot, dialogue, query, user_handler, short_cache, msg_id),
    fields(
        chat_id = %dialogue.chat_id(),
    )
)]
pub async fn subscriptions_callback(
    bot: Throttle<Bot>,
    dialogue: ShortBotDialogue,
    query: CallbackQuery,
    user_handler: Arc<UserHandler>,
    short_cache: Arc<ShortCache>,
    msg_id: Option<MessageId>,
) -> HandlerResult {
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
    let lang_code = user_lang_code(&user_id, user_handler.clone(), None).await;

    bot.answer_callback_query(query.id).await?;

    let callback_payload = query.data.unwrap();

    match callback_payload.as_str() {
        // Firs stage
        "add_subscriptions" => {
            add_subscriptions(
                &bot,
                &dialogue,
                user_handler,
                short_cache,
                &user_id,
                msg_id.unwrap(),
            )
            .await?;
        }
        "delete_subscriptions" => {
            delete_subscriptions(
                &bot,
                &dialogue,
                user_handler,
                short_cache,
                &user_id,
                msg_id.unwrap(),
            )
            .await?;
        }
        "clear_subscriptions" => {
            clear_subscriptions(&bot, &dialogue, user_handler.clone(), &user_id, msg_id).await?;
            dialogue.exit().await?
        }
        "exit" => {
            if let Some(msg_id) = msg_id {
                bot.delete_message(dialogue.chat_id(), msg_id).await?;
            }
            dialogue.exit().await?;
        }
        // Second stage
        _ => {
            if let Some(state) = dialogue.get().await? {
                match state {
                    State::AddSubscriptions { msg_id } => {
                        info!("State add subscriptions");
                        let result = user_handler
                            .add_subscriptions(
                                &user_id,
                                Subscriptions::try_from(&callback_payload).unwrap(),
                            )
                            .await;
                        if let Some(msg_id) = msg_id {
                            if let Err(e) = result {
                                error!("Error found: {e}");
                                bot.edit_message_text(dialogue.chat_id(), msg_id, "Error found while adding your new subscription. Try again later.").await?;
                            } else {
                                bot.edit_message_text(
                                    dialogue.chat_id(),
                                    msg_id,
                                    format!("{callback_payload} added to your subscriptions"),
                                )
                                .await?;
                            }
                        } else if let Err(e) = result {
                            error!("Error found: {e}");
                            bot.send_message(
                                dialogue.chat_id(),
                                "Error found while adding your new subscription. Try again later.",
                            )
                            .await?;
                        } else {
                            bot.send_message(
                                dialogue.chat_id(),
                                format!("{callback_payload} added to your subscriptions"),
                            )
                            .await?;
                        }
                    }
                    State::DeleteSubscriptions { msg_id } => {
                        info!("State delete subscriptions");
                        let result = user_handler
                            .remove_subscriptions(
                                &user_id,
                                Subscriptions::try_from(&callback_payload).unwrap(),
                            )
                            .await;
                        if let Some(msg_id) = msg_id {
                            if let Err(e) = result {
                                error!("Error found: {e}");
                                bot.edit_message_text(
                                    dialogue.chat_id(),
                                    msg_id,
                                    _error_found(&lang_code),
                                )
                                .await?;
                            } else {
                                bot.edit_message_text(
                                    dialogue.chat_id(),
                                    msg_id,
                                    format!("{callback_payload} removed from your subscriptions"),
                                )
                                .await?;
                            }
                        } else if let Err(e) = result {
                            error!("Error found: {e}");
                            bot.send_message(dialogue.chat_id(), _error_found(&lang_code))
                                .await?;
                        } else {
                            bot.send_message(
                                dialogue.chat_id(),
                                format!("{callback_payload} removed from your subscriptions"),
                            )
                            .await?;
                        }
                    }
                    _ => {
                        info!("How the hell I reached this point?");
                    }
                }
            } else if let Some(msg_id) = msg_id {
                bot.delete_message(dialogue.chat_id(), msg_id).await?;
            }

            dialogue.exit().await?
        }
    }

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

/// Internal function to handle adding new subscriptions
pub(crate) async fn add_subscriptions(
    bot: &Throttle<Bot>,
    dialogue: &ShortBotDialogue,
    user_handler: Arc<UserHandler>,
    short_cache: Arc<ShortCache>,
    user_id: &UserId,
    msg_id: MessageId,
) -> HandlerResult {
    let lang_code = &user_lang_code(user_id, user_handler.clone(), None).await;
    let ibex_market = short_cache.ibex35_listing().await?;

    let msg_id = bot
        .edit_message_text(
            dialogue.chat_id(),
            msg_id,
            _select_ticker_message(lang_code),
        )
        .reply_markup(tickers_grid_keyboard(&ibex_market))
        .await?
        .id;

    dialogue
        .update(State::AddSubscriptions {
            msg_id: Some(msg_id),
        })
        .await?;

    Ok(())
}

/// Internal function to handle adding new subscriptions
pub(crate) async fn delete_subscriptions(
    bot: &Throttle<Bot>,
    dialogue: &ShortBotDialogue,
    user_handler: Arc<UserHandler>,
    _short_cache: Arc<ShortCache>,
    user_id: &UserId,
    msg_id: MessageId,
) -> HandlerResult {
    //let ibex_market = short_cache.ibex35_listing().await?;
    let lang_code = &user_lang_code(user_id, user_handler.clone(), None).await;

    let user_cfg: UserConfig = match user_handler.user_config(user_id).await {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Error while obtaining user's config from the DB: {e}");
            return Ok(());
        }
    };

    let current_subscriptions = match user_handler.subscriptions(user_id).await {
        Ok(s) => s,
        Err(e) => {
            error!("Error found while retrieving user's subscriptions: {e}");
            bot.send_message(dialogue.chat_id(), _error_found(lang_code))
                .await?;
            return Ok(());
        }
    };

    if let Some(subscriptions) = current_subscriptions {
        let subscriptions = Into::<Vec<String>>::into(subscriptions);
        let subscriptions_ref = subscriptions
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<&str>>();

        let (message, keyboard_markup) = if user_cfg.prefer_tickers {
            (
                _select_ticker_message(lang_code),
                small_buttons_grid_keyboard(subscriptions_ref.as_slice()),
            )
        } else {
            todo!()
        };

        let msg_id = bot
            .edit_message_text(dialogue.chat_id(), msg_id, message)
            .reply_markup(keyboard_markup)
            .await?
            .id;

        dialogue
            .update(State::DeleteSubscriptions {
                msg_id: Some(msg_id),
            })
            .await?;
    } else {
        bot.send_message(
            dialogue.chat_id(),
            "¡No tienes subscripciones que eliminar!",
        )
        .await?;
        dialogue.exit().await?;
    }

    Ok(())
}

pub(crate) async fn clear_subscriptions(
    bot: &Throttle<Bot>,
    dialogue: &ShortBotDialogue,
    user_handler: Arc<UserHandler>,
    user_id: &UserId,
    msg_id: Option<MessageId>,
) -> HandlerResult {
    let lang_code = &user_lang_code(user_id, user_handler.clone(), None).await;
    if let Some(msg_id) = msg_id {
        bot.delete_message(dialogue.chat_id(), msg_id).await?;
    }
    bot.send_message(
        dialogue.chat_id(),
        if lang_code == "es" {
            "🧹 Borrando tus subscripciones ..."
        } else {
            "🧹 Wiping your current subscriptions ..."
        },
    )
    .await?;

    let current_subscriptions = match user_handler.subscriptions(user_id).await {
        Ok(s) => s,
        Err(e) => {
            error!("Error found while retrieving the subscriptions of the user: {e}");
            bot.send_message(dialogue.chat_id(), _error_message(lang_code))
                .await?;
            dialogue.exit().await?;
            return Ok(());
        }
    };

    if let Some(subscriptions) = current_subscriptions {
        let error = user_handler
            .remove_subscriptions(user_id, subscriptions)
            .await;

        if let Err(e) = error {
            error!("Error found while removing subscriptions of the user: {e}");
            bot.send_message(dialogue.chat_id(), _error_message(lang_code))
                .await?;
            dialogue.exit().await?;
            return Ok(());
        }
    } else {
        bot.send_message(
            dialogue.chat_id(),
            if lang_code == "es" {
                "⁉️ No hay subscripciones que borrar"
            } else {
                "⁉️ There are no subscriptions to delete"
            },
        )
        .disable_notification(true)
        .await?;
        dialogue.exit().await?;
    }

    Ok(())
}

fn _error_message(lang_code: &str) -> &str {
    match lang_code {
        "es" => "🚒 Ha ocurrido un error, por favor, inténtalo más tarde",
        _ => "🚒 An error was found, please try again later",
    }
}

fn _select_ticker_message(lang_code: &str) -> String {
    match lang_code {
        "es" => String::from("Selecciona un ticker:"),
        _ => String::from("Select a ticker:"),
    }
}

fn _select_company_message(lang_code: &str) -> String {
    match lang_code {
        "es" => String::from("Selecciona una empresa:"),
        _ => String::from("Choose a company:"),
    }
}

fn _select_starting_letter(lang_code: &str) -> String {
    match lang_code {
        "es" => String::from("Selecciona la letra por la que empieza el nombre de la empresa:"),
        _ => String::from("Choose the starting letter for the company's name:"),
    }
}

fn _error_found(lang_code: &str) -> String {
    match lang_code {
        "es" => String::from("👩‍🔧 Se ha detectado un error. Inténtalo de nuevo más tarde."),
        _ => String::from("👩‍🔧 An error ocurred. Please, try again later."),
    }
}
