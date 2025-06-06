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
use chrono::Utc;
use sqlx::MySqlPool;
use std::{sync::Arc, time::Duration};
use teloxide::types::UserId;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// Transmission timeout for the cache maintenance channel (milliseconds).
const DEFAULT_CACHE_TX_CHANNEL_TIMEOUT: u64 = 1;

/// Handler for the management of the client's metadata.
pub struct ClientHandler {
    /// DB pool reference.
    db_conn: MySqlPool,
    /// Reference to the cache.
    cache: Arc<Cache>,
    /// Transmitter for the channel to communicate with the cache handler.
    tx_channel: mpsc::Sender<String>,
}

// TODO: Logic for last_update
impl ClientHandler {
    pub fn new(db_conn: MySqlPool, cache: Arc<Cache>, sender: mpsc::Sender<String>) -> Self {
        ClientHandler {
            db_conn,
            cache,
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
            Some(metadata) => Ok(metadata.access_level),
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
                    let now = Some(Utc::now());
                    metadata.last_access = now;
                    metadata.last_update = now;
                    self.db_mark_as_registered(client_id).await?;
                } else {
                    warn!("User {} is already registered", client_id.0);
                }
            }
            None => {
                self.db_register_client(client_id, false).await?;
                let now = Some(Utc::now());
                let dummy_meta = ClientMeta {
                    registered: true,
                    last_access: now,
                    last_update: now,
                    ..Default::default()
                };

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

    /// Method that modifies the access level of a client.
    pub async fn modify_access_level(
        &self,
        client_id: &UserId,
        access: BotAccess,
    ) -> Result<(), ClientError> {
        match self.cache.data.get_mut(&client_id.0).await {
            Some(mut meta) => {
                meta.access_level = access;
                self.notify_cache_handler(client_id).await;
                Ok(())
            }
            None => {
                warn!("The user ID is not registered as a client of the bot");
                Err(ClientError::ClientNotRegistered)
            }
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

    /// TC: Get the access level of an unregistered user.
    ///
    /// # Description
    ///
    /// ## Pre
    ///
    /// - The cache is empty.
    /// - There are no client records in the DB.
    ///
    /// ## Inputs
    ///
    /// - A random user ID.
    /// - An empty cache.
    ///
    /// ## TC
    ///
    /// Any unregistered user of the bot must get assigned a level of access `BotAccess::Free`.
    ///
    /// ## Result
    ///
    /// The user identified by the random ID has an access level = `BotAccess::Free`.
    #[sqlx::test]
    async fn access_level_tc1(pool: MySqlPool) {
        Lazy::force(&TRACING);

        let mut source = random::default(42);
        let client_id = UserId {
            0: source.read::<u64>(),
        };
        let expected_access_level = BotAccess::Free;
        let (_, client_handler) = ClientObjectsBuilder::new(pool.clone()).build();

        let access_test = client_handler
            .access_level(&client_id)
            .await
            .expect("Error trying to get access level");
        assert_eq!(
            access_test, expected_access_level,
            "Access level should be free"
        );
    }

    /// TC: Get the access level of a registered user.
    ///
    /// # Description
    ///
    /// ## Pre
    ///
    /// - The cache contains a registered client.
    /// - The register user API is implemented and tested.
    ///
    /// ## Inputs
    ///
    /// - A registered user's ID.
    ///
    /// ## TC
    ///
    /// Test that the stored value is retrieved from user stored in the DB.
    ///
    /// ## Result
    ///
    /// TThe retrieved access level matches the value stored in the DB.
    #[sqlx::test]
    async fn access_level_tc2(pool: MySqlPool) -> sqlx::Result<()> {
        Lazy::force(&TRACING);

        let mut source = random::default(42);

        let (_, client_handler) = ClientObjectsBuilder::new(pool.clone()).build();
        let access_level_table = vec![
            (
                UserId {
                    0: source.read::<u64>(),
                },
                BotAccess::Free,
            ),
            (
                UserId {
                    0: source.read::<u64>(),
                },
                BotAccess::Limited,
            ),
            (
                UserId {
                    0: source.read::<u64>(),
                },
                BotAccess::Unlimited,
            ),
            (
                UserId {
                    0: source.read::<u64>(),
                },
                BotAccess::Admin,
            ),
        ];

        // Modify the access level of the test clients according to the table.
        for (id, ba) in access_level_table.iter() {
            client_handler
                .register_client(id)
                .await
                .expect("Failed to register client");
            client_handler
                .modify_access_level(id, *ba)
                .await
                .expect("Failed to modify access");
        }

        // Test
        for (id, access) in access_level_table.iter() {
            assert_eq!(
                *access,
                client_handler
                    .access_level(id)
                    .await
                    .expect("Error trying to get access level")
            );
        }

        Ok(())
    }

    /// TC1: Get the access level of an unregistered user.
    ///
    /// # Description
    ///
    /// ## Pre
    ///
    /// - The cache is empty.
    /// - There are no client records in the DB.
    ///
    /// ## Inputs
    ///
    /// - A random user ID.
    /// - An empty cache.
    ///
    /// ## TC
    ///
    /// Any unregistered user of the bot must get assigned a level of access `BotAccess::Free`.
    ///
    /// ## Result
    ///
    /// The user identified by the random ID has an access level = `BotAccess::Free`.
    #[sqlx::test]
    async fn register_tc1(pool: MySqlPool) {
        Lazy::force(&TRACING);

        // Test setup
        let mut source = random::default(42);
        let client_id = UserId {
            0: source.read::<u64>(),
        };
        let (_, client_handler) = ClientObjectsBuilder::new(pool.clone()).build();

        // Register a new client using the API
        client_handler
            .register_client(&client_id)
            .await
            .expect("Failed to register a new client");

        // Extract it using a raw SQL query
        let db_client = match sqlx::query!("SELECT * FROM BotClient WHERE id = ?", client_id.0)
            .fetch_optional(&pool)
            .await
            .expect("Failed to retrieve registered client")
        {
            Some(row) => ClientMeta {
                registered: if row.registered > 0 { true } else { false },
                access_level: BotAccess::from_str(&row.access).unwrap(),
                subscriptions: match row.subscriptions {
                    Some(s) => Some(
                        Subscriptions::try_from(&s)
                            .expect("Failed to parse a subscription list from the DB"),
                    ),
                    None => None,
                },
                last_access: row.last_access,
                last_update: None,
                created_at: row.created_at,
            },
            None => panic!("Failed to register a new client"),
        };

        // Ensure the base fields hold the expected values
        assert_eq!(db_client.registered, true);
        assert_eq!(db_client.access_level, BotAccess::Free);
        assert_eq!(db_client.subscriptions, None);
        assert!(db_client.created_at.is_some());
    }

    /// TC2: Attempt to register an existing client
    ///
    /// # Description
    ///
    /// ## Pre
    ///
    /// - The cache is empty.
    /// - The client is already registered as a hard-client.
    ///
    /// ## Inputs
    ///
    /// - A random user ID.
    ///
    /// ## TC
    ///
    /// Attempt to register twice a user of the bot.
    ///
    /// ## Result
    ///
    /// The API must return OK and only one entry is registered in the DB.
    #[sqlx::test]
    async fn register_tc2(pool: MySqlPool) {
        Lazy::force(&TRACING);

        // Test setup
        let mut source = random::default(42);
        let client_id = UserId {
            0: source.read::<u64>(),
        };
        let (_, client_handler) = ClientObjectsBuilder::new(pool.clone()).build();

        // Register a new client using the API
        client_handler
            .register_client(&client_id)
            .await
            .expect("Failed to register a new client");

        // Register a new client using the API
        client_handler
            .register_client(&client_id)
            .await
            .expect("Failed to register a new client");

        // Extract it using a raw SQL query
        let clients = sqlx::query!("SELECT * FROM BotClient")
            .fetch_all(&pool)
            .await
            .expect("Failed to retrieve registered client");

        assert_eq!(clients.len(), 1);
    }

    /// TC1: Check that a new client id is not registered.
    ///
    /// # Description
    ///
    /// ## Pre
    ///
    /// - The cache is empty.
    ///
    /// ## Inputs
    ///
    /// - A unregistered user's ID.
    /// - An empty cache.
    ///
    /// ## TC
    ///
    /// Test that the API detects new IDs, and proceeds to register these as _soft-clients_.
    /// After that, if we repeat the check, we must receive the same result.
    ///
    /// ## Result
    ///
    /// We receive `false` for a unregistered user's ID.
    #[sqlx::test]
    async fn is_registered_tc1(pool: MySqlPool) {
        Lazy::force(&TRACING);

        let mut source = random::default(42);
        let client_id = UserId {
            0: source.read::<u64>(),
        };
        let (_, client_handler) = ClientObjectsBuilder::new(pool.clone()).build();

        assert_eq!(
            false,
            client_handler
                .is_registered(&client_id)
                .await
                .expect("Failed to check ID")
        );
        assert_eq!(
            false,
            client_handler
                .is_registered(&client_id)
                .await
                .expect("Failed to check ID")
        );
    }
}
