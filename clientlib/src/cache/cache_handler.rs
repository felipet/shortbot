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

//! This module includes the objects that maintain the client cache coherent to the content kept in
//! the data base.

use crate::{BotAccess, Subscriptions};
use crate::{Cache, ClientError, ClientMeta, UserId};
use chrono::Utc;
use sqlx::{Executor, MySqlPool};
use std::sync::Mutex;
use std::{str::FromStr, sync::Arc};
use tokio::sync::mpsc;
use tracing::{error, info, instrument};
// use chrono::{DateTime, Utc};

/// Handles maintenance tasks to keep coherent the client cache respect to the data base.
///
/// # Description
///
/// This `struct` decouples the maintenance tasks from the client handler [crate::ClientHandler].
///
/// The operations supported by this object are:
/// - Load the cache using the content stored in the DB.
/// - Store the content of the cache into the DB.
/// - Refresh staled cache content.
///
/// The object includes a method that receives requests through a _mpsc_ channel. That's the
/// main access point to the supported features: when a cache client detects staled content and
/// needs to refresh it ASAP, it should deliver a message to refresh such content.
pub struct CacheHandler {
    /// DB pool.
    db_conn: MySqlPool,
    /// Consumer side of the MPSC channel.
    rx_channel: mpsc::Receiver<String>,
    cache: Arc<Cache>,
    /// List of IDs whose metadata needs refreshing.
    update_queue: Mutex<Vec<UserId>>,
    /// Threshold to trigger the process of the queue tasks.
    queue_service: usize,
}

/// Commands supported by [CacheHandler].
///
/// # Description
///
/// [CacheHandler] allows requesting some maintenance tasks over the cache using message passing.
/// Producers of the channel can issue commands defined by this `enum` to trigger actions on the
/// handler.
///
/// ## Commands
///
/// Commands are `String`s that contain one of the variants of the `enum` [CacheHandlerCmd]. The
/// variants shall convert to lowercase `String`s. Two formats are expected:
///
/// - Single command format: `<command string>`.
/// - Command + payload: `<command string>:<payload>`.
///
/// The character `:` is used to delimit the command from the payload (when needed). The payload is
/// passed raw to the next layer. See the variants docs to read more information about the payloads.
///
/// ## Supported actions
///
/// 1. **Ping**: a dummy command to ensure the handler is alive and healthy. It is also used to trigger
///    delayed tasks that are queued.
/// 2. **Update**: a command that requests to update some content of the cache. This command includes
///    payload, which shall contain a cache key. The update might get queued.
#[derive(Default, Debug, Clone)]
enum CacheHandlerCmd {
    #[default]
    Ping,
    Update(String),
    Save,
    Load,
    Stop,
}

impl From<String> for CacheHandlerCmd {
    fn from(value: String) -> Self {
        let raw_cmd = value.split(":").collect::<Vec<&str>>();
        let (cmd, payload) = if raw_cmd.len() > 1 {
            (raw_cmd[0], Some(raw_cmd[1]))
        } else {
            (raw_cmd[0], None)
        };
        match cmd {
            "ping" => CacheHandlerCmd::Ping,
            "update" => CacheHandlerCmd::Update(payload.unwrap_or_default().to_owned()),
            _ => CacheHandlerCmd::Stop,
        }
    }
}

impl CacheHandler {
    pub fn new(
        db_conn: MySqlPool,
        rx_channel: mpsc::Receiver<String>,
        cache: Arc<Cache>,
        use_queue: usize,
    ) -> Self {
        CacheHandler {
            db_conn,
            rx_channel,
            cache,
            update_queue: Mutex::new(Vec::new()),
            queue_service: use_queue,
        }
    }

    pub async fn start(&mut self) -> Result<(), ClientError> {
        while let Some(msg) = self.rx_channel.recv().await {
            match CacheHandlerCmd::from(msg.to_string()) {
                CacheHandlerCmd::Ping => {
                    info!("Ping command received");
                    if self.update_queue.lock().unwrap().len() >= self.queue_service {
                        self.process_queue().await?;
                    }
                }
                CacheHandlerCmd::Save => {
                    info!("Save command received");
                    self.save_cache().await?;
                }
                CacheHandlerCmd::Load => {
                    info!("Load command received");
                    self.load_cache().await?;
                }
                CacheHandlerCmd::Update(u) => {
                    info!("Update command received for {u}");
                    let id: u64 = u.parse().unwrap();
                    {
                        self.update_queue.lock().unwrap().push(id);
                    }
                }
                _ => {
                    info!("Stop command received. Graceful shutdown the cache handler");
                    self.save_cache().await?;
                    return Ok(());
                }
            }
        }

        Ok(())
    }

    /// Save the content of the cache to permanent memory.
    #[instrument(name = "Process the queued update requests", skip(self))]
    pub async fn process_queue(&self) -> Result<(), ClientError> {
        let update_list: Vec<u64>;

        // Lock the list and make a clone, so the lock doesn't hold and blocks other threads
        // that might push new update jobs to the new list.
        {
            let mut queue = self.update_queue.lock().unwrap();
            update_list = queue.clone();
            queue.clear();
        }

        for item in update_list.into_iter() {
            let meta = match self.cache.data.get(&item).await {
                Some(metadata) => Arc::new(metadata.clone()),
                None => {
                    error!("ID from the client list {item} not present in the cache");
                    return Err(ClientError::CacheIncongruence);
                }
            };

            self.update_db_entry(item, &meta).await?;
        }

        Ok(())
    }

    /// Save the content of the cache to permanent memory.
    #[instrument(name = "Save the in-memory cache content", skip(self))]
    pub async fn save_cache(&self) -> Result<(), ClientError> {
        // Hold the lock: prevent new registries or changes in the client list.
        {
            let client_list = self.cache.clients.lock().await;

            for client in client_list.iter() {
                let meta = match self.cache.data.get(client).await {
                    Some(m) => Arc::new(m.clone()),
                    None => {
                        error!("The ID {client} wasn't included in the cache");
                        return Err(ClientError::CacheIncongruence);
                    }
                };

                self.update_db_entry(*client, &meta).await?;
            }
        }
        // Lock released

        Ok(())
    }

    /// Load the content of the cache from permanent memory.
    #[instrument(name = "Load the in-memory cache content", skip(self))]
    pub async fn load_cache(&self) -> Result<(), ClientError> {
        let raw_cache = sqlx::query!("SELECT * from BotClient")
            .fetch_all(&self.db_conn)
            .await?;

        for r in raw_cache {
            // Lock the client list
            {
                self.cache.clients.lock().await.push(r.id);
                self.cache
                    .data
                    .insert(
                        r.id,
                        ClientMeta {
                            registered: r.registered > 0,
                            access_level: BotAccess::from_str(&r.access).map_err(|_| {
                                ClientError::UnknownDbError(format!(
                                    "Wrong format in BotAccess field for {}",
                                    r.id,
                                ))
                            })?,
                            subscriptions: match r.subscriptions {
                                Some(s) => Some(Subscriptions::try_from(s).map_err(|_| {
                                    ClientError::UnknownDbError(format!(
                                        "Wrong format in BotAccess field for {}",
                                        r.id,
                                    ))
                                })?),
                                None => None,
                            },
                            last_access: r.last_access,
                            last_update: Some(Utc::now()),
                            created_at: r.created_at,
                        },
                    )
                    .await;
            }
        }

        Ok(())
    }

    /// Update the entry in the DB of a client of the bot.
    #[instrument(name = "Update client entry in the DB", skip(self))]
    async fn update_db_entry(
        &self,
        client_id: UserId,
        new_data: &ClientMeta,
    ) -> Result<(), ClientError> {
        self.db_conn
            .execute(sqlx::query!(
                "UPDATE BotClient
                SET registered = ?, access = ?, subscriptions = ?, created_at = ?, last_access = ?
                WHERE id = ?",
                new_data.registered,
                new_data.access_level.to_string(),
                match new_data.subscriptions.clone() {
                    Some(s) => Some(s.to_string()),
                    None => None,
                },
                new_data.created_at,
                new_data.last_update,
                client_id,
            ))
            .await?;

        Ok(())
    }

    #[instrument(name = "Retrieve the entry of a client from the DB", skip(self))]
    async fn retrieve_db_entry(&self, client_id: UserId) -> Result<ClientMeta, ClientError> {
        let row = sqlx::query!("SELECT * FROM BotClient WHERE id = ?", client_id)
            .fetch_optional(&self.db_conn)
            .await?;

        match row {
            Some(r) => Ok(ClientMeta {
                registered: r.registered > 0,
                access_level: BotAccess::from_str(&r.access).map_err(|_| {
                    ClientError::UnknownDbError(format!(
                        "Wrong format in BotAccess field for {client_id}",
                    ))
                })?,
                subscriptions: match r.subscriptions {
                    Some(s) => Some(Subscriptions::try_from(s).map_err(|_| {
                        ClientError::UnknownDbError(format!(
                            "Wrong format in BotAccess field for {client_id}",
                        ))
                    })?),
                    None => None,
                },
                last_access: r.last_access,
                last_update: Some(Utc::now()),
                created_at: r.created_at,
            }),
            None => Err(ClientError::ClientNotRegistered),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::{ClientObjectsBuilder, Subscriptions};
    use once_cell::sync::Lazy;
    use random::Source;
    use teloxide::types::UserId;
    use tokio::time::sleep;
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

    #[sqlx::test]
    async fn update_db_entry(pool: MySqlPool) -> sqlx::Result<()> {
        Lazy::force(&TRACING);
        let mut source = random::default(42);
        let client_id = UserId {
            0: source.read::<u64>(),
        };
        let initial_meta = ClientMeta {
            registered: true,
            access_level: BotAccess::Free,
            subscriptions: None,
            last_access: None,
            last_update: None,
            created_at: None,
        };
        let test_meta = ClientMeta {
            registered: true,
            access_level: BotAccess::Limited,
            subscriptions: Some(
                Subscriptions::try_from(["SAN"].as_slice()).expect("Failed to build subscriptions"),
            ),
            last_access: Some(Utc::now()),
            last_update: Some(Utc::now()),
            created_at: None,
        };

        pool.execute(sqlx::query!(
            "INSERT INTO BotClient VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, ?)",
            client_id.0,
            initial_meta.registered,
            initial_meta.access_level.to_string(),
            match initial_meta.subscriptions {
                Some(s) => Some(s.to_string()),
                None => None,
            },
            initial_meta.last_access,
        ))
        .await?;

        let (cache_handler, _) = ClientObjectsBuilder::new(pool.clone()).build();

        cache_handler
            .update_db_entry(client_id.0, &test_meta)
            .await
            .expect("Failed to update the DB entry");

        let entry = cache_handler
            .retrieve_db_entry(client_id.0)
            .await
            .expect("Failed to retrieve DB entry");

        assert_eq!(entry, test_meta);

        Ok(())
    }

    #[sqlx::test]
    async fn load(pool: MySqlPool) -> sqlx::Result<()> {
        Lazy::force(&TRACING);
        let mut source = random::default(42);
        let client_ids = source.iter().take(50).collect::<Vec<u64>>();

        let (cache_handler, client_handler) = ClientObjectsBuilder::new(pool.clone()).build();

        for id in client_ids {
            client_handler
                .register_client(&UserId(id))
                .await
                .expect("Failed to register a client");
        }

        cache_handler
            .save_cache()
            .await
            .expect("Failed to save the cache");

        // Now, load it.

        let (cache_handler_test, _) = ClientObjectsBuilder::new(pool.clone()).build();

        cache_handler_test
            .load_cache()
            .await
            .expect("Failed to load the cache");

        // Bot caches must be equal
        let cache_initial = cache_handler.cache.clone();
        let cache_loaded = cache_handler_test.cache.clone();

        assert_eq!(
            cache_initial.clients.lock().await.len(),
            cache_loaded.clients.lock().await.len()
        );

        Ok(())
    }

    #[sqlx::test]
    async fn update(pool: MySqlPool) -> sqlx::Result<()> {
        Lazy::force(&TRACING);
        let mut source = random::default(42);
        let client_ids = source.iter().take(10).collect::<Vec<u64>>();
        let (tx, rx) = tokio::sync::mpsc::channel(20);

        let (mut cache_handler, client_handler) = ClientObjectsBuilder::new(pool.clone())
            .with_channel(tx.clone(), rx)
            .build();

        let task = tokio::spawn(async move { cache_handler.start().await });

        for id in client_ids {
            client_handler
                .register_client(&teloxide::types::UserId(id))
                .await
                .expect("Failed to register the client");
            tx.send(format!("update:{id}"))
                .await
                .expect("Failed to send message to the handler");
        }

        tx.send("ping".to_owned())
            .await
            .expect("Failed to send ping");

        sleep(Duration::from_millis(10)).await;

        tx.send("stop".to_owned())
            .await
            .expect("Failed to send message to the handler");

        let _ = task.await.expect("Failed to graceful close the handler");

        Ok(())
    }
}
