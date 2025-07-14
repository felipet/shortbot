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

//! Module with the handler for the webhook endpoint of Shortbot.
//!
//! # Description
//!
//! **Shortbot** features a webhook that allows triggering administration tasks. The following tasks are
//! included:
//!
//! - Send a broadcast message to the users of the bot.
//!
//! Requests must include a bearer token to authenticate the source of the request.
//!
//! ## Broadcast messages
//!
//! Broadcast messages are useful to notify about the new features of a new release of the bot, or to notify
//! about a maintenance period, etc.
//!
//! When the bot receives a broadcast message request via webhook, it obtains a list of the registered users
//! of the bot, and sends the message to them.
//!
//! Messages support HTML formatting. For a complete list of the supported HMTL tags, see
//! [Teloxide Parsemode](https://docs.rs/teloxide/latest/teloxide/types/enum.ParseMode.html#html-style).
//!
//! Example of message:
//!
//! ```bash
//! curl -X GET 'http://localhost:9602/adm/webhook' \
//!   -H 'Authorization: Basic token' \
//!   -H 'Content-Type: application/json' \
//!   -d '{"req_type":"BroadcastAllMessage","req_payload":"{\"message_en\":\"Eng message\",\"message_es\":\"Spa message\"}"}'
//! ```

use crate::{WebServerState, errors::BotError, users::UserConfig};
use axum::{
    Json,
    extract::State,
    http::{HeaderName, header::HeaderMap},
};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use teloxide::{
    prelude::*,
    types::{ChatId, ParseMode},
};
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub enum RequestType {
    BroadcastAllMessage,
    BroadcastSilentMessage,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebhookRequest {
    req_type: RequestType,
    req_payload: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BroadcastMessage {
    pub message_en: String,
    pub message_es: String,
}

fn auth_client(headers: HeaderMap, token: &SecretString) -> Result<(), BotError> {
    let raw_token = match headers.get(HeaderName::from_lowercase(b"authorization").unwrap()) {
        Some(header) => header,
        None => {
            warn!("Webhook request received without authentication token");
            return Err(BotError::MissingCredentials);
        }
    };

    let (auth_type, token_client) = match raw_token.to_str() {
        Ok(r) => (
            r.split(" ").collect::<Vec<_>>()[0],
            r.split(" ").collect::<Vec<_>>()[1],
        ),
        Err(_) => return Err(BotError::InvalidToken),
    };

    if auth_type.to_ascii_lowercase() != "basic" {
        error!("Invalid authorization schema provided ({auth_type})");
        return Err(BotError::WrongCredentials);
    }

    if token.expose_secret() != token_client {
        error!("Invalid authorization token provided");
        return Err(BotError::WrongCredentials);
    }

    Ok(())
}

pub async fn webhook_handler(
    headers: HeaderMap,
    State(state): State<WebServerState>,
    Json(payload): Json<WebhookRequest>,
) -> Result<String, BotError> {
    info!("Webhook request received to send a broadcast message");
    debug!("Broadcast message: {}", payload.req_payload.clone());

    // Check the credentials of the client.
    auth_client(headers, &state.webhook_token)?;

    let (message_es, message_en) =
        match serde_json::from_str::<BroadcastMessage>(&payload.req_payload) {
            Ok(m) => (m.message_es, m.message_en),
            Err(e) => {
                error!("Error while deserialising the broadcast message: {e}");
                return Err(BotError::WrongMessageFormat);
            }
        };

    if payload.req_type == RequestType::BroadcastAllMessage
        || payload.req_type == RequestType::BroadcastSilentMessage
    {
        let users_list = match state
            .user_handler
            .list_users(payload.req_type == RequestType::BroadcastAllMessage)
            .await
        {
            Ok(ul) => ul,
            Err(e) => {
                error!("Error found while requesting a list of registered users: {e}");
                return Err(BotError::InternalServerError);
            }
        };

        for user in users_list.into_iter() {
            let user_cfg: UserConfig = match state.user_handler.user_config(&UserId(user)).await {
                Ok(cfg) => cfg,
                Err(e) => {
                    error!("Error found while extracting user's ({user}) config from DB: {e}");
                    continue;
                }
            };

            if let Err(e) = state
                .bot
                .send_message(
                    ChatId(user as i64),
                    if user_cfg.lang_code == "es" {
                        &message_es
                    } else {
                        &message_en
                    },
                )
                .parse_mode(ParseMode::Html)
                .await
            {
                error!("Error while sending broadcast message to user {user}: {e}");
            }
        }
    } else {
        warn!("Webhook feature not implemented");
    }

    Ok("Broadcast message sent successfully".to_owned())
}
