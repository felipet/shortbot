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

//! Module with logic for the handler that sends updates to users with subscriptions.

use crate::{
    ShortCache,
    endpoints::short_report,
    users::{Subscriptions, UserHandler, user_lang_code},
};
use std::{error::Error, sync::Arc};
use teloxide::{
    Bot,
    adaptors::Throttle,
    payloads::SendMessageSetters,
    prelude::Requester,
    types::{ChatId, ParseMode, UserId},
};
use tokio::sync::mpsc::Receiver;
use tracing::{debug, error, info, instrument, warn};

#[instrument(
    name = "Update Handler",
    skip(bot, user_handler, short_cache, update_buffer_rx)
)]
pub async fn update_handler(
    bot: Throttle<Bot>,
    user_handler: Arc<UserHandler>,
    short_cache: Arc<ShortCache>,
    mut update_buffer_rx: Receiver<String>,
) -> Result<(), Box<dyn Error>> {
    tokio::task::spawn(async move {
        while let Some(msg) = update_buffer_rx.recv().await {
            let split = msg.split(":").collect::<Vec<&str>>();
            let user_handler = user_handler.clone();
            let short_cache = short_cache.clone();

            if split.len() < 2 {
                error!("Unknown command received: {msg}");
                continue;
            }

            let (cmd, payload) = (split[0], split[1]);

            match cmd {
                "upd" => {
                    info!("Request for notification of short positions updates received");
                    let users_with_subscriptions = match user_handler.list_users(true).await {
                        Ok(list) => {
                            let mut users = Vec::new();

                            for user_id in list {
                                let user_id = UserId(user_id);
                                let user_subscriptions = match user_handler
                                    .subscriptions(&user_id)
                                    .await
                                {
                                    Ok(subs) => subs,
                                    Err(e) => {
                                        error!(
                                            "Error found while retrieving user subscriptions: {e}"
                                        );
                                        break;
                                    }
                                };

                                if let Some(user_subscriptions) = user_subscriptions {
                                    users.push((user_id, user_subscriptions));
                                }
                            }

                            users
                        }
                        Err(e) => {
                            error!("Error found while retrieving user list: {e}");
                            continue;
                        }
                    };

                    let tickers = match Subscriptions::try_from(payload) {
                        Ok(p) => p,
                        Err(e) => {
                            error!("Incorrect format of the updates payload: {e}");
                            continue;
                        }
                    };

                    info!("Starting to notify users with subscriptions");
                    match notify_users(
                        bot.clone(),
                        user_handler,
                        short_cache,
                        users_with_subscriptions,
                        tickers,
                    )
                    .await
                    {
                        Ok(_) => info!("Users with subscriptions successfully notified"),
                        Err(e) => {
                            error!("Error found while notifying users: {e}");
                            continue;
                        }
                    };
                }
                _ => {
                    warn!("Not implemented command requested: {msg}");
                    continue;
                }
            }
        }
    });

    Ok(())
}

async fn notify_users(
    bot: Throttle<Bot>,
    user_handler: Arc<UserHandler>,
    short_cache: Arc<ShortCache>,
    user_list: Vec<(UserId, Subscriptions)>,
    tickers: Subscriptions,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    for ticker in tickers {
        for (user, user_subscriptions) in user_list.iter() {
            debug!("Processing updates for the ticker {ticker}");
            if user_subscriptions.is_subscribed(&[&ticker]) {
                debug!("Sending notification to the user {}", user.0);
                let lang_code = &user_lang_code(user, user_handler.clone(), None).await;
                let chat_id = ChatId(user.0 as i64);
                // Will be the casting an issue? Why they chose unsigned types for User's ID whilst signed for Chat's
                // IDs? A total nonsense.
                bot.send_message(chat_id, _short_update_msg(lang_code))
                    .parse_mode(ParseMode::Html)
                    .await?;

                short_report(&bot, chat_id, short_cache.clone(), lang_code, &ticker).await?;
            }
        }
    }

    Ok(())
}

fn _short_update_msg(lang_code: &str) -> String {
    match lang_code {
        "es" => "¡<b>⚠️ Una posición en corto ha sufrido cambios!</b>".to_owned(),
        _ => "<b>⚠️ A short positions got updated!</b>".to_owned(),
    }
}
