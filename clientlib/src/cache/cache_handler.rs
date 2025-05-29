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
use chrono::{DateTime, TimeDelta, Utc};
use sqlx::MySqlPool;
use std::{str::FromStr, sync::Arc};
use tokio::sync::mpsc;
use tracing::{info, warn};
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
    update_queue: Vec<UserId>,
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
            update_queue: Vec::new(),
            queue_service: use_queue,
        }
    }

    pub async fn start(&mut self) -> Result<(), ClientError> {
        // while let Some(msg) = self.rx_channel.recv().await {
        //     match CacheHandlerCmd::from(msg.to_string()) {
        //         CacheHandlerCmd::Ping => {
        //             info!("Ping received");
        //             if self.update_queue.len() >= self.queue_service {
        //                 self.process_queue().await?;
        //             }
        //         }
        //         _ => {
        //             info!("Stop command received. Graceful shutdown the cache handler");
        //             //TODO: save the cache
        //             return Ok(());
        //         }
        //     }
        // }

        Ok(())
    }

    pub async fn process_queue(&mut self) -> Result<(), ClientError> {
        // for key in self.update_queue.iter() {
        //     let db_entry = sqlx::query!(
        //         r#"
        //             SELECT registered, access, subscriptions, created_at, last_access
        //             FROM BotClient
        //             WHERE id = ?
        //         "#,
        //         key
        //     )
        //     .fetch_optional(&self.db_conn)
        //     .await?;

        //     let db_meta = match db_entry {
        //         Some(record) => ClientMeta {
        //             registered: record.registered != 0,
        //             access_level: BotAccess::from_str(&record.access).unwrap_or(BotAccess::Free),
        //             subscriptions: Some(Subscriptions::try_from(
        //                 record.subscriptions.unwrap_or("".to_owned()),
        //             )?),
        //             last_access: record.last_access,
        //             last_update: None,
        //             created_at: record.created_at,
        //         },
        //         None => return Err(ClientError::UnknownDbError("????".to_owned())),
        //     };

        // let cache_meta = self.cache.data.get(key).await.unwrap_or(default)
        // }

        // self.update_queue.clear();

        Ok(())
    }

    pub async fn update_cache(&mut self) -> Result<(), ClientError> {
        Ok(())
    }

    // pub async fn save_cache(&mut self) -> Result<(), ClientError> {
    //     for client in self.cache.client_list.iter() {
    //         match self.cache.get_mut(client).await {
    //             Some(mut metadata) => {
    //                 self.db_conn
    //                     .execute(sqlx::query!(
    //                         r#"
    //                         UPDATE BotClient
    //                         SET registered = ?, access = ?, subscriptions = ?, last_access = ?
    //                         WHERE id = ?
    //                     "#,
    //                         metadata.registered,
    //                         metadata.access_level.to_string(),
    //                         metadata.subscriptions.to_string(),
    //                         metadata.last_access.to_string(),
    //                         client.0,
    //                     ))
    //                     .await?;
    //                 metadata.last_update = Utc::now();
    //             }
    //             None => warn!("Missing cache metadata for client: {}", client.0),
    //         }
    //     }

    //     Ok(())
    // }
}
