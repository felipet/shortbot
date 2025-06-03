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

use crate::{BotAccess, Cache, ClientError, ClientMeta, Subscriptions};
use chrono::{TimeDelta, Utc};
use sqlx::MySqlPool;
use std::{str::FromStr, sync::Arc, time::Duration};
use teloxide::types::UserId;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

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
                    self.notify_cache_handler(client_id).await;
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

    /// Method that returns whether an user is registered as a _hard-client_.
    ///
    /// # Description
    ///
    /// This method checks if the given client ID was registered previously in the DB. When a new
    /// client is detected, this method calls [ClientHandler::db_register_client] and proceeds to
    /// register the user as a _soft-client_.
    pub async fn is_registered(&self, client_id: &UserId) -> Result<bool, ClientError> {
        match self.cache.data.get(&client_id.0).await {
            Some(metadata) => Ok(metadata.registered),
            None => {
                info!("New user detected. Proceeding to register it (soft)");
                self.db_register_client(client_id, true).await?;
                // Load a dummy entry in the cache for this client.
                self.cache
                    .data
                    .insert(client_id.0, ClientMeta::default())
                    .await;
                self.notify_cache_handler(client_id).await;
                // Add the client ID to the clients array.
                {
                    self.cache.clients.lock().await.push(client_id.0);
                }
                Ok(false)
            }
        }
    }

    /// Method that registers an user as a _hard-client_ client of the bot.
    ///
    /// # Description
    ///
    /// The method checks if the user was auto-registered before proceeding to the register process. In that case,
    /// the _auto_ flag is set to `true` and the access time is updated.
    /// Otherwise, a full register process is triggered.
    pub async fn register_client(&self, client_id: &UserId) -> Result<(), ClientError> {
        match self.cache.data.get_mut(&client_id.0).await {
            Some(mut metadata) => {
                if !metadata.registered {
                    metadata.registered = true;
                    metadata.last_access = Some(Utc::now());
                    self.db_mark_as_registered(client_id).await?;
                } else {
                    warn!("User {} is already registered", client_id.0);
                }
            }
            None => {
                self.db_register_client(client_id, false).await?;
                let mut dummy_meta = ClientMeta::default();
                dummy_meta.registered = true;
                self.cache.data.insert(client_id.0, dummy_meta).await;
                info!("User {} registered in the DB", client_id.0);
            }
        }

        // Add the client ID to the clients array.
        {
            self.cache.clients.lock().await.push(client_id.0);
        }

        Ok(())
    }

    /// Method that retrieves the subscriptions of the client.
    pub async fn subscriptions(
        &self,
        client_id: &UserId,
    ) -> Result<Option<Subscriptions>, ClientError> {
        match self.cache.data.get(&client_id.0).await {
            Some(metadata) => match &metadata.subscriptions {
                Some(s) => Ok(Some(s.clone())),
                None => Ok(None),
            },
            None => {
                warn!("Attempt to get subscriptions of a client non-registered");
                Err(ClientError::ClientNotRegistered)
            }
        }
    }

    /// Method that adds tickers to the subscription list of the client.
    pub async fn add_subscriptions(
        &self,
        client_id: &UserId,
        subscriptions: Subscriptions,
    ) -> Result<(), ClientError> {
        match self.cache.data.get_mut(&client_id.0).await {
            Some(mut metadata) => {
                if metadata.subscriptions.is_none() {
                    metadata.subscriptions = Some(subscriptions);
                    self.notify_cache_handler(client_id).await;
                } else {
                    *metadata.subscriptions.as_mut().unwrap() += subscriptions;
                }
                info!("The client {} added new subscriptions", client_id.0);
            }
            None => {
                warn!("Attempt to subscribe items to a client non-registered");
                return Err(ClientError::ClientNotRegistered);
            }
        };

        Ok(())
    }

    /// Method that removes tickers from the subscription list of the client.
    pub async fn remove_subscriptions(
        &self,
        client_id: &UserId,
        subscriptions: Subscriptions,
    ) -> Result<(), ClientError> {
        match self.cache.data.get_mut(&client_id.0).await {
            Some(mut metadata) => {
                if metadata.subscriptions.is_none() {
                    warn!("Attempt to remove subscriptions from a client non-registered");
                } else {
                    let subs = metadata.subscriptions.as_mut().unwrap();
                    *subs -= subscriptions;

                    if subs.is_empty() {
                        metadata.subscriptions = None;
                    }

                    self.notify_cache_handler(client_id).await;
                    info!("The client {} removed subscriptions", client_id.0);
                }
            }
            None => {
                warn!("Attempt to remove subscriptions from a client non-registered");
                return Err(ClientError::ClientNotRegistered);
            }
        };

        Ok(())
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
            !auto_register,
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

    async fn notify_cache_handler(&self, client_id: &UserId) {
        let _ = self
            .tx_channel
            .send_timeout(
                format!("update:{}", client_id.0),
                Duration::from_millis(DEFAULT_CACHE_TX_CHANNEL_TIMEOUT),
            )
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ClientObjectsBuilder, Subscriptions};
    use once_cell::sync::Lazy;
    use random::Source;
    use tracing::{Level, subscriber::set_global_default};
    use tracing_subscriber::FmtSubscriber;

    static TRACING: Lazy<()> = Lazy::new(|| {
        if std::env::var("TEST_LOG").is_ok() {
            let level =
                std::env::var("TEST_LOG").expect("Failed to read the content of TEST_LOG var");
            let level = match level.as_str() {
                "info" => Some(Level::INFO),
                "debug" => Some(Level::DEBUG),
                "warn" => Some(Level::WARN),
                "error" => Some(Level::ERROR),
                &_ => None,
            };

            if level.is_some() {
                let subscriber = FmtSubscriber::builder()
                    .with_max_level(level.unwrap())
                    .finish();
                set_global_default(subscriber).expect("Failed to set subscriber.");
            }
        }
    });

    /// TC: Insert a subscription for a registered client.
    ///
    /// # Description
    ///
    /// ## Pre
    ///
    /// - The cache includes a client hard-registered.
    ///
    /// ## Inputs
    ///
    /// - A random user ID.
    ///
    /// ## TC
    ///
    /// This TC inserts a new subscription to a registered user that had no previous subscriptions.
    /// Then, it attempts to register the same subscription.
    ///
    /// Finally, adds another subscription.
    ///
    /// ## Result
    ///
    /// The test subscriptions match the retrieved subscriptions from the cache. Duplicated values are ignored.
    #[sqlx::test]
    async fn add(pool: MySqlPool) -> sqlx::Result<()> {
        Lazy::force(&TRACING);

        let mut source = random::default(42);
        let client_id = UserId {
            0: source.read::<u64>(),
        };
        let (_, client_handler) = ClientObjectsBuilder::new(pool.clone()).build();

        // Seed a client into the cache.
        client_handler
            .register_client(&client_id)
            .await
            .expect("Failed to seed a client");

        // First: let's insert a new subscription.
        let test_subscriptions = Subscriptions::try_from(["SAN"].as_ref())
            .expect("Failed to create a subscriptions object");
        client_handler
            .add_subscriptions(&client_id, test_subscriptions.clone())
            .await
            .expect("Failed to add new subscriptions");

        let metadata = client_handler
            .cache
            .data
            .get(&client_id.0)
            .await
            .expect("Failed to retrieve cached client")
            .clone();

        assert_eq!(metadata.subscriptions.unwrap(), test_subscriptions);

        // Second: let's try to insert the same subscription.
        client_handler
            .add_subscriptions(&client_id, test_subscriptions.clone())
            .await
            .expect("Failed to add new subscriptions");

        let metadata = client_handler
            .cache
            .data
            .get(&client_id.0)
            .await
            .expect("Failed to retrieve cached client")
            .clone();

        assert_eq!(metadata.subscriptions.unwrap(), test_subscriptions);

        // Third: let's insert an array of subscriptions this time.
        let mut test_subscriptions = Subscriptions::try_from(["BBVA", "SAB"].as_ref())
            .expect("Failed to create a subscriptions object");

        client_handler
            .add_subscriptions(&client_id, test_subscriptions.clone())
            .await
            .expect("Failed to add new subscriptions");

        let cache_subscriptions = client_handler
            .cache
            .data
            .get(&client_id.0)
            .await
            .expect("Failed to retrieve cached client")
            .clone()
            .subscriptions
            .unwrap();
        // SAN was added in the previous test.
        test_subscriptions.add_subscriptions(&["SAN"]);

        assert_eq!(cache_subscriptions, test_subscriptions);

        Ok(())
    }

    /// TC: Remove a subscription for a registered client.
    ///
    /// # Description
    ///
    /// ## Pre
    ///
    /// - The cache includes a client hard-registered.
    /// - The client has some subscriptions.
    /// - [ClientHandler::add_subscriptions] works.
    ///
    /// ## Inputs
    ///
    /// - A random user ID.
    ///
    /// ## TC
    ///
    /// This TC attempts to remove a subscription that exists, another that doesn't exist and a series of
    /// subscriptions at once.
    ///
    /// ## Result
    ///
    /// The test subscriptions match the retrieved subscriptions from the cache.
    #[sqlx::test]
    async fn remove(pool: MySqlPool) -> sqlx::Result<()> {
        Lazy::force(&TRACING);

        let mut source = random::default(42);
        let client_id = UserId {
            0: source.read::<u64>(),
        };
        let (_, client_handler) = ClientObjectsBuilder::new(pool.clone()).build();

        // Seed a client into the cache.
        client_handler
            .register_client(&client_id)
            .await
            .expect("Failed to seed a client");

        // First: let's insert a new subscription.
        let mut test_subscriptions = Subscriptions::try_from(["SAN", "ENG", "REP", "IAG"].as_ref())
            .expect("Failed to create a subscriptions object");
        client_handler
            .add_subscriptions(&client_id, test_subscriptions.clone())
            .await
            .expect("Failed to add new subscriptions");

        // Time to attempt to remove a existing subscription.
        let to_remove = Subscriptions::try_from(["ENG"].as_ref())
            .expect("Failed to create a subscriptions object");
        test_subscriptions -= &to_remove;

        client_handler
            .remove_subscriptions(&client_id, to_remove.clone())
            .await
            .expect("Failed to remove subscriptions");

        let cache_subscriptions = client_handler
            .cache
            .data
            .get(&client_id.0)
            .await
            .expect("Failed to retrieve cached client")
            .clone()
            .subscriptions
            .unwrap();

        assert_eq!(cache_subscriptions, test_subscriptions);

        // Let's try again but this time the subscription won't be there.
        client_handler
            .remove_subscriptions(&client_id, to_remove)
            .await
            .expect("Failed to remove subscriptions");

        let cache_subscriptions = client_handler
            .cache
            .data
            .get(&client_id.0)
            .await
            .expect("Failed to retrieve cached client")
            .clone()
            .subscriptions
            .unwrap();

        assert_eq!(cache_subscriptions, test_subscriptions);

        // And multiple subscriptions at once.
        let to_remove = Subscriptions::try_from(["REP", "IAG"].as_ref())
            .expect("Failed to create a subscriptions object");
        test_subscriptions -= &to_remove;

        client_handler
            .remove_subscriptions(&client_id, to_remove.clone())
            .await
            .expect("Failed to remove subscriptions");

        let cache_subscriptions = client_handler
            .cache
            .data
            .get(&client_id.0)
            .await
            .expect("Failed to retrieve cached client")
            .clone()
            .subscriptions
            .unwrap();

        assert_eq!(cache_subscriptions, test_subscriptions);

        Ok(())
    }

    /// TC: Retrieve the subscriptions of a client.
    ///
    /// # Description
    ///
    /// ## Pre
    ///
    /// - The cache includes a client hard-registered.
    /// - The client has some subscriptions.
    /// - [ClientHandler::add_subscriptions] works.
    ///
    /// ## Inputs
    ///
    /// - A random user ID.
    ///
    /// ## TC
    ///
    /// This TC retrieves the subscriptions of a client.
    ///
    /// ## Result
    ///
    /// The test subscriptions match the retrieved subscriptions from the cache.
    #[sqlx::test]
    async fn retrieve(pool: MySqlPool) -> sqlx::Result<()> {
        Lazy::force(&TRACING);

        let mut source = random::default(42);
        let client_id = UserId {
            0: source.read::<u64>(),
        };
        let (_, client_handler) = ClientObjectsBuilder::new(pool.clone()).build();

        // Seed a client into the cache.
        client_handler
            .register_client(&client_id)
            .await
            .expect("Failed to seed a client");

        let test_subscriptions = Subscriptions::try_from(["SAN", "REP"].as_ref())
            .expect("Failed to create a subscriptions object");
        client_handler
            .add_subscriptions(&client_id, test_subscriptions.clone())
            .await
            .expect("Failed to add new subscriptions");

        let subscriptions = client_handler
            .subscriptions(&client_id)
            .await
            .expect("Failed to retrieve the subscriptions of the client");

        assert_eq!(subscriptions, Some(test_subscriptions));

        // Now, let's wipe those subscriptions and check that we get a None.
        client_handler
            .remove_subscriptions(&client_id, subscriptions.unwrap())
            .await
            .expect("Failed to remove the existing subscriptions");

        let subscriptions = client_handler
            .subscriptions(&client_id)
            .await
            .expect("Failed to retrieve the subscriptions of the client");

        assert!(subscriptions.is_none());

        Ok(())
    }
}
