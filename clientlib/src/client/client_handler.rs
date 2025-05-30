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

//! Module that includes all the logic related to the management of the client's metadata.
//!
//! # Description
//!
//! The `struct` [ClientHandler] is the API for external modules that aim to request or modify data related to
//! clients of the bot. It makes transparent the usage of the cache, so external modules don't need to know whether
//! the information is available in the cache or not.
//!
//! The [ClientHandler] main goal is to serve other modules as handler to access a client's information with a
//! minimum latency (so the bot keeps responsive).
//!
//! It won't take part of cache maintenance tasks to avoid reducing the performance of the handler. That task,
//! and other related to the handling of the cache are implemented in the module [crate::cache]. [ClientHandler]
//! only signals the cache handler when a refresh is needed.

use crate::{BotAccess, Cache, ClientError, Subscriptions};
use chrono::{TimeDelta, Utc};
use sqlx::MySqlPool;
use std::{str::FromStr, sync::Arc, time::Duration};
use teloxide::types::UserId;
use tokio::sync::mpsc;
use tracing::{debug, warn};

/// Minimum allowed cache refresh request interval (minutes). Avoids using low values from external config files.
const DEFAULT_MINIMUM_CACHE_REFRESH: i64 = 60;

/// Transmission timeout for the cache maintenance channel (milliseconds).
const DEFAULT_CACHE_TX_CHANNEL_TIMEOUT: u64 = 1;

/// Handler for the management of the client's metadata.
pub struct ClientHandler {
    /// DB pool reference.
    db_conn: MySqlPool,
    /// Reference to the cache.
    cache: Arc<Cache>,
    /// When to consider a data in the cache expired.
    cache_expiry: TimeDelta,
    /// Transmitter for the channel to communicate with the cache handler.
    tx_channel: mpsc::Sender<String>,
}

impl ClientHandler {
    pub fn new(
        db_conn: MySqlPool,
        cache: Arc<Cache>,
        cache_expiry: TimeDelta,
        sender: mpsc::Sender<String>,
    ) -> Self {
        let cache_expiry = if cache_expiry < TimeDelta::minutes(DEFAULT_MINIMUM_CACHE_REFRESH) {
            warn!(
                "Client cache refresh value below threshold. Using the default value ({DEFAULT_MINIMUM_CACHE_REFRESH} minutes)"
            );
            TimeDelta::minutes(DEFAULT_MINIMUM_CACHE_REFRESH)
        } else {
            cache_expiry
        };

        ClientHandler {
            db_conn,
            cache,
            cache_expiry,
            tx_channel: sender,
        }
    }

    /// Method that retrieves the access level of a Telegram user.
    ///
    /// # Description
    ///
    /// This method acts as high level API to retrieve the access level ([BotAccess]) of a client of the bot.
    pub async fn access_level(&self, client_id: &UserId) -> Result<BotAccess, ClientError> {
        match self.cache.data.get(&client_id.0).await {
            Some(metadata) => {
                // The Default DateTime is Jan 1970.
                if Utc::now() - metadata.last_update.unwrap_or_default() > self.cache_expiry {
                    self.tx_channel
                        .send_timeout(
                            format!("update:{}", client_id.0),
                            Duration::from_millis(DEFAULT_CACHE_TX_CHANNEL_TIMEOUT),
                        )
                        .await;
                    debug!("Access level metadata was expired");
                    self.db_access_level(&client_id).await
                } else {
                    debug!("Access level metadata was cached");
                    Ok(metadata.access_level)
                }
            }
            None => {
                debug!("Access level requested for client not registered");
                Ok(BotAccess::Free)
            }
        }
    }

    /// Method that refreshes the last access time of the user.
    ///
    /// # Description
    ///
    /// This method is meant to be called anytime a handler of the bot is called from an user. On each call,
    /// the access time will get updated.
    ///
    /// If the method is called using a client ID which wasn't registered before in the DB, it will call
    /// the register method in auto-mode.
    pub async fn refresh_access(&self, _client_id: &UserId) -> Result<(), ClientError> {
        unimplemented!("Refresh access API not implemented")
    }

    /// Method that returns whether an user is registered as a client.
    pub async fn is_registered(&self, _client_id: &UserId) -> Result<bool, ClientError> {
        unimplemented!("Is registered client API not implemented")
    }

    /// Method that registers an user as a client.
    pub async fn register_client(&self, _client_id: &UserId) -> Result<(), ClientError> {
        unimplemented!("Register client API not implemented")
    }

    /// Method that retrieves the subscriptions of the client.
    pub async fn subscriptions(&self, _client_id: &UserId) -> Result<Subscriptions, ClientError> {
        unimplemented!("Subscriptions API not implemented")
    }

    /// Method that adds tickers to the subscription list of the client.
    pub async fn add_subscriptions(
        &self,
        _client_id: &UserId,
    ) -> Result<Subscriptions, ClientError> {
        unimplemented!("Subscriptions API not implemented")
    }

    /// Method that removes tickers from the subscription list of the client.
    pub async fn remove_subscriptions(
        &self,
        _client_id: &UserId,
    ) -> Result<Subscriptions, ClientError> {
        unimplemented!("Subscriptions API not implemented")
    }

    async fn db_access_level(&self, client_id: &UserId) -> Result<BotAccess, ClientError> {
        let row = sqlx::query!("SELECT access FROM BotClient WHERE id = ?", client_id.0)
            .fetch_optional(&self.db_conn)
            .await?;

        match row {
            Some(row) => Ok(BotAccess::from_str(&row.access).unwrap_or(BotAccess::Free)),
            None => Ok(BotAccess::Free),
        }
    }

    async fn db_is_registered(&self, client_id: &UserId) -> Result<bool, ClientError> {
        let row = sqlx::query!("SELECT registered FROM BotClient WHERE id = ?", client_id.0)
            .fetch_optional(&self.db_conn)
            .await?;

        match row {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    async fn db_register_client(
        &self,
        client_id: &UserId,
        auto_register: bool,
    ) -> Result<(), ClientError> {
        sqlx::query!(
            "INSERT INTO BotClient VALUES (?, ?, ?, NULL, CURRENT_TIMESTAMP(), NULL)",
            client_id.0,
            auto_register,
            BotAccess::Free.to_string(),
        )
        .execute(&self.db_conn)
        .await?;

        Ok(())
    }

    async fn db_mark_as_registered(&self, client_id: &UserId) -> Result<(), ClientError> {
        sqlx::query!(
            "UPDATE BotClient SET registered = true WHERE id = ?",
            client_id.0
        )
        .execute(&self.db_conn)
        .await?;

        Ok(())
    }

    async fn db_modify_access_level(
        &self,
        client_id: &UserId,
        access_level: BotAccess,
    ) -> Result<(), ClientError> {
        sqlx::query!(
            "UPDATE BotClient SET access = ? WHERE id = ?",
            access_level.to_string(),
            client_id.0
        )
        .execute(&self.db_conn)
        .await?;

        Ok(())
    }

    async fn db_update_access_time(&self, client_id: &UserId) -> Result<(), ClientError> {
        sqlx::query!(
            "UPDATE BotClient SET last_access = CURRENT_TIMESTAMP() WHERE id = ?",
            client_id.0
        )
        .execute(&self.db_conn)
        .await?;

        Ok(())
    }

    async fn db_subscriptions(&self, client_id: &UserId) -> Result<Subscriptions, ClientError> {
        let row = sqlx::query!(
            "SELECT subscriptions FROM BotClient WHERE id = ?",
            client_id.0
        )
        .fetch_one(&self.db_conn)
        .await?;

        match row.subscriptions {
            Some(tickers) => Subscriptions::try_from(tickers),
            None => Ok(Subscriptions::default()),
        }
    }

    async fn db_add_subscriptions(
        &self,
        subscriptions: &[&str],
        client_id: &UserId,
    ) -> Result<Subscriptions, ClientError> {
        let mut tickers = self.db_subscriptions(client_id).await?;

        tickers.add_subscriptions(subscriptions);

        self.db_update_subscriptions(
            Into::<Vec<String>>::into(tickers.clone())
                .iter()
                .map(|e| e.as_str())
                .collect::<Vec<_>>()
                .as_slice(),
            client_id,
        )
        .await?;

        Ok(tickers)
    }

    async fn db_remove_subscriptions(
        &self,
        subscriptions: &[&str],
        client_id: &UserId,
    ) -> Result<Subscriptions, ClientError> {
        let mut tickers = self.db_subscriptions(client_id).await?;

        tickers.remove_subscriptions(subscriptions);

        self.db_update_subscriptions(
            Into::<Vec<String>>::into(tickers.clone())
                .iter()
                .map(|e| e.as_str())
                .collect::<Vec<_>>()
                .as_slice(),
            client_id,
        )
        .await?;

        Ok(tickers)
    }

    async fn db_update_subscriptions(
        &self,
        subscriptions: &[&str],
        client_id: &UserId,
    ) -> Result<(), ClientError> {
        sqlx::query!(
            "UPDATE BotClient SET subscriptions = ? WHERE id = ?",
            subscriptions.join(";"),
            client_id.0
        )
        .execute(&self.db_conn)
        .await?;

        Ok(())
    }
}
