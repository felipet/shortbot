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

//! Representation of a cache of bot client's metadata.

use crate::{ClientMeta, UserId};
use std::sync::Arc;
use tokio::sync::Mutex;
use whirlwind::ShardMap;

/// Object that represents a cache for bot client's metadata.
///
/// # Description
///
/// The cache keeps the most relevant information of a client of the bot in the main memory. This would avoid an
/// excessive use of the data base, which would end in a poor global performance.
///
/// The data that is cached is defined by the `struct` [crate::client_handler::ClientMeta]. Thus this object is a
/// mere data container of [crate::client_handler::ClientMeta] entries. The functionality of the cache is implemented
/// across two handlers: [crate::ClientHandler], for the logic related to the modification and retrieval of the
/// metadata; and [crate::CacheHandler], for the low level management tasks. See those object's documentation for
/// more details.
///
/// # Main features
///
/// - The cache is _async-ready_ which means it allows being shared across multiple threads. Concurrent readers are
///   supported, but only a writer is allowed at a time. To minimize contention, the internal Hashmap is sharded.
/// - Support for custom data expiry (implemented on [crate::ClientHandler]).
/// - Automatic loading/saving data from/to the data base (implemented on [crate::CacheHandler]).
///
/// # Implementation
///
/// [whirlwind::ShardMap] is the choice for the concurrent HashMap with sharding. It's fast, thread-safe and features
/// a simple API. However, it does not allow iterating over the entries.
///
/// That is a needed feature in order to perform some maintenance tasks. To cope with that problem, a
/// [Vec] was selected to keep a simple unordered list of the registered clients. This list is only used when
/// iteration over the existing clients is needed. The rest of the operations is issued to the HashMap.
#[derive(Clone)]
pub struct Cache {
    pub data: ShardMap<UserId, ClientMeta>,
    pub clients: Arc<Mutex<Vec<UserId>>>,
}

impl Default for Cache {
    fn default() -> Self {
        Self {
            data: ShardMap::new(),
            clients: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Cache {
    pub fn new(shards: usize) -> Self {
        Self {
            data: ShardMap::with_shards(shards),
            ..Default::default()
        }
    }
}
