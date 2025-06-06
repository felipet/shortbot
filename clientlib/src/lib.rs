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

//! Library that includes all the logic related to the management of clients of the bot.
//!
//! # Description
//!
//! This library includes modules that are meant to implement all the logic related to the management of users of
//! the bot (clients).
//!
//! Clients' metadata is stored in a data base. However, to speed-up the application a cache subsystem has been
//! developed to keep a coherent copy of the clients' data base in main memory. The application expects no more
//! than hundreds of users, so keeping their metadata in main memory won't demand a lot of resources.
//!
//! How the data is stored is transparent to the bot, which only needs to interface with the module
//! [crate::ClientHandler]. This is the API to access all the logic related to client's management.
//!
//! A minimal setup is required by the startup code. A cache needs to be created at startup and all the handlers
//! need to configured. This process is automated using [crate::ClientObjectsBuilder].
//!
//! ### Example of Use
//!
//! TODO: add an example with the setup of the client management subsystem.
//!
//! ## Organisation
//!
//! The crate includes two main modules:
//!
//! 1. [crate::cache] which is in charge of the cache subsystem.
//! 2. [crate::client] which is in charge of the management logic to keep metadata related to clients.
//!
//! ## What Is a Client of the Bot
//!
//! Users of the bot become _clients_ when they start using some of the advanced features of the bot. This means
//! regular users don't get fully registered in the client DB. All the advanced features relate to those features that
//! need some sort of memory storage.
//!
//! The main purpose of the crate is to free the bot's logic of all the stuff related to remember what tickers is a
//! client subscribed at, or if a client expects to receive some sort of periodical information, and so on.
//!
//! Anyway, all the users that happen to use the bot, at least once, get registered. The main purpose of this feature
//! is enabling later analysis of the bot's usage and how many users are actively using the bot.
//! So any user that uses the bot gets _soft-registered_ or _auto-registered_. Users become _hard-registered_ when
//! they start using advanced features.
//!
//! ## Why This Is a Separated Crate?
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
//! # How To Develop This Library
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

use chrono::Duration;
use sqlx::MySqlPool;
use std::{str::FromStr, sync::Arc};
use thiserror::Error;
use tokio::sync::mpsc::{self, Receiver, Sender};

/// Client management module.
mod client {
    pub(crate) mod client_handler;
    pub(crate) mod client_meta;
    pub(crate) mod subscriptions;
}

pub(crate) use client::client_meta::ClientMeta;
pub use client::{client_handler::ClientHandler, subscriptions::Subscriptions};

/// Cache management module.
mod cache {
    pub mod cache_handler;
    pub mod cache_type;
}

pub use cache::cache_handler::CacheHandler;
pub use cache::cache_type::Cache;

/// The backend is not expected to run using too many threads. Keep this low unless
/// the number of threads escalates enough.
const DEFAULT_SHARDS: usize = 4;

/// The most important metadata is the access type, and that is not expected to get
/// updated more frequently than once per day.
const DEFAULT_CACHE_EXPIRICY: Duration = Duration::days(1);

/// Capacity of the MPSC channel that allows sending tasks to the [CacheHandler].
const DEFAULT_BUFFER_SIZE: usize = 20;

/// Cache handler force using a queue of 10 tasks.
const DEFAULT_CACHE_TASK_QUEUE: usize = 10;

/// User ID internal type. See [teloxide::types::UserId].
pub type UserId = u64;

/// This enum represents the access level of a bot client.
///
/// # Description
///
/// The access level is used to determine the level of access to the bot's features for each client.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub enum BotAccess {
    #[default]
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

/// Builder object that construct all the objects related to the bot client's DB & cache.
pub struct ClientObjectsBuilder {
    db_conn: MySqlPool,
    cache: Option<Cache>,
    shards: Option<usize>,
    cache_expiricy: Option<chrono::Duration>,
    channel_size: Option<usize>,
    channel: Option<(Sender<String>, Receiver<String>)>,
    cache_queue_size: usize,
}

impl ClientObjectsBuilder {
    pub fn new(db_conn: MySqlPool) -> Self {
        ClientObjectsBuilder {
            db_conn,
            cache: None,
            shards: None,
            cache_expiricy: None,
            channel_size: None,
            channel: None,
            cache_queue_size: 0,
        }
    }

    pub fn build(self) -> (CacheHandler, ClientHandler) {
        // Build an MPSC channel when not provided.
        let (tx_channel, rx_channel) = self.channel.unwrap_or(mpsc::channel(
            self.channel_size.unwrap_or(DEFAULT_BUFFER_SIZE),
        ));

        // Build a Cache when not provided.
        let cache = Arc::new(self.cache.unwrap_or(whirlwind::ShardMap::with_shards(
            self.shards.unwrap_or(DEFAULT_SHARDS),
        )));

        // Create an instance of ClientHandler.
        let client_handler = ClientHandler::new(
            self.db_conn.clone(),
            cache.clone(),
            self.cache_expiricy.unwrap_or(DEFAULT_CACHE_EXPIRICY),
            tx_channel,
        );

        // Create an instance of CacheHandler.
        let cache_handler = CacheHandler::new(
            self.db_conn.clone(),
            rx_channel,
            cache,
            self.cache_queue_size,
        );

        (cache_handler, client_handler)
    }

    pub fn with_cache(mut self, cache: Cache) -> Self {
        self.cache = Some(cache);

        self
    }

    pub fn with_cache_size(mut self, size: usize) -> Self {
        self.cache_queue_size = size;

        self
    }

    pub fn with_shards(mut self, shards: usize) -> Self {
        self.shards = Some(shards);

        self
    }

    pub fn with_channel(mut self, sender: Sender<String>, receiver: Receiver<String>) -> Self {
        self.channel = Some((sender, receiver));

        self
    }

    pub fn with_channel_size(mut self, size: usize) -> Self {
        self.channel_size = Some(size);

        self
    }
}

impl From<sqlx::Error> for ClientError {
    fn from(value: sqlx::Error) -> Self {
        ClientError::UnknownDbError(value.to_string())
    }
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
