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

//! `ClientLib` library crate.
//!
//! # Description
//!
//! This crate splits all the logic that relies on the MariaDB backend. The main purpose of this separation
//! is to enable SQLx to properly analyze and build the queries of the application. The bot features
//! several DB backends, which is not supported by SQLx as of today.
//!
//! The most straightforward workaround is to split the code into several crates, each one of them connects to
//! a specific DB backend. This way, SQLx can analyze the code and build the queries properly.
//!
//! All the code related to handling client's preferences, subscriptions or any other information related to them
//! is included in this crate as it relies on the MariaDB backend.
//!
//! ## How To Develop This Library
//!
//! In order to build successfully all the code of the application, the following procedure must be followed:
//!
//! For each crate of the workspace:
//!
//! 1. Set up the environment variables for connecting to the DB backend, either via `export DATABASE_URL` or using
//!    `.env` files.
//! 2. Build the crate using `cargo build`.
//! 3. Run `cargo sqlx prepare` to generate the SQLx prepared queries.
//!
//! Remember to commit those files to the repository.
//!
//! After that, the whole workspace can be built using `cargo build`, but we need to run SQLx in offline mode:
//! `export SQLX_OFFLINE=true`.

use std::future::Future;
use std::str::FromStr;
use teloxide::types::UserId;
use thiserror::Error;

mod client_handler;
mod subscriptions;

pub use client_handler::ClientHandler;
pub use subscriptions::Subscriptions;

/// This enum represents the access level of a bot client.
///
/// # Description
///
/// The access level is used to determine the level of access to the bot's features for each client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BotAccess {
    Free,
    Limited,
    Unlimited,
    Admin,
}

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Wrong subscription string format")]
    WrongSubscriptionString(String),
    #[error("Unknown error from the DB server")]
    UnknownDbError(String),
}

impl From<sqlx::Error> for ClientError {
    fn from(value: sqlx::Error) -> Self {
        ClientError::UnknownDbError(value.to_string())
    }
}

pub trait ClientDbHandler {
    fn is_registered(
        &self,
        client_id: UserId,
    ) -> impl Future<Output = Result<bool, ClientError>> + Send;
    fn register_client(
        &self,
        client_id: UserId,
        auto_register: bool,
    ) -> impl Future<Output = Result<(), ClientError>> + Send;
    fn access_level(
        &self,
        client_id: UserId,
    ) -> impl Future<Output = Result<BotAccess, ClientError>> + Send;

    fn update_access_time(
        &self,
        client_id: UserId,
    ) -> impl Future<Output = Result<(), ClientError>> + Send;

    fn modify_access_level(
        &self,
        client_id: UserId,
        access_level: BotAccess,
    ) -> impl Future<Output = Result<(), ClientError>> + Send;

    fn mark_as_registered(
        &self,
        client_id: UserId,
    ) -> impl Future<Output = Result<(), ClientError>> + Send;

    fn subscriptions(
        &self,
        client_id: UserId,
    ) -> impl Future<Output = Result<Subscriptions, ClientError>> + Send;

    fn add_subscriptions(
        &self,
        subscriptions: &[&str],
        client_id: UserId,
    ) -> impl Future<Output = Result<Subscriptions, ClientError>> + Send;

    fn remove_subscriptions(
        &self,
        subscriptions: &[&str],
        client_id: UserId,
    ) -> impl Future<Output = Result<Subscriptions, ClientError>> + Send;
}

impl FromStr for BotAccess {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "free" => Ok(BotAccess::Free),
            "limited" => Ok(BotAccess::Limited),
            "unlimited" => Ok(BotAccess::Unlimited),
            "admin" => Ok(BotAccess::Admin),
            _ => Err("Invalid BotAccess type"),
        }
    }
}

impl std::fmt::Display for BotAccess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BotAccess::Free => write!(f, "free"),
            BotAccess::Limited => write!(f, "limited"),
            BotAccess::Unlimited => write!(f, "unlimited"),
            BotAccess::Admin => write!(f, "admin"),
        }
    }
}
