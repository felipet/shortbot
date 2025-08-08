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

//! Custom error types.
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("missing stock information in the DB")]
    MissingStockInfo(String),
    #[error("unknown db error")]
    Unknown(String),
    #[error("unknown error from the QuestDB dataserver")]
    UnknownQdb(String),
    #[error("error from Valkey DB server")]
    UnknownValkey(String),
}

#[derive(Error, Debug)]
pub enum UserError {
    #[error("Wrong subscription string format")]
    WrongSubscriptionString(String),
    #[error("The user ID is not registered")]
    ClientNotRegistered,
    #[error("Subscription limit reached")]
    ClientLimitReached,
    #[error("serialisation error")]
    SerialisationError(String),
}

#[derive(Debug)]
pub enum BotError {
    WrongCredentials,
    MissingCredentials,
    InvalidToken,
    WrongMessageFormat,
    InternalServerError,
}

impl IntoResponse for BotError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            BotError::WrongCredentials => (StatusCode::UNAUTHORIZED, "Wrong credentials"),
            BotError::MissingCredentials => (StatusCode::BAD_REQUEST, "Missing credentials"),
            BotError::InvalidToken => (StatusCode::BAD_REQUEST, "Invalid token"),
            BotError::WrongMessageFormat => {
                (StatusCode::BAD_REQUEST, "Wrong format of the payload")
            }
            BotError::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Wrong format of the payload",
            ),
        };
        let body = Json(serde_json::json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

pub(crate) fn error_message(lang_code: &str) -> &str {
    match lang_code {
        "es" => "🚒 Ha ocurrido un error, por favor, inténtalo más tarde",
        _ => "🚒 An error was found, please try again later",
    }
}
