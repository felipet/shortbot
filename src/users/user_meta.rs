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

//! Module that contains definitions related to metadata for users of the bot.

use crate::users::{BotAccess, Subscriptions};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Metadata of a bot's user.
///
/// # Description
///
/// This `struct` represents a data object for a user of the bot. It contains
/// data that is stored in a DB, but also data that is only needed for the
/// internal use of the cache.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct UserMeta {
    /// Identifies the level of access of the client. See [BotAccess].
    pub access_level: BotAccess,
    /// List of subscriptions of the client.
    pub subscriptions: Option<Subscriptions>,
    /// Timestamp of the last access of the client to the bot.
    pub last_access: DateTime<Utc>,
    /// Timestamp of the client register process.
    pub created_at: DateTime<Utc>,
}

impl UserMeta {
    pub fn new() -> Self {
        UserMeta {
            access_level: BotAccess::Free,
            created_at: Utc::now(),
            ..Default::default()
        }
    }
}

impl PartialEq for UserMeta {
    /// Compare two instances of [UserMeta].
    ///
    /// # Description
    ///
    /// Two instances _are the same_ if all these members are the same on both sides:
    /// - [UserMeta::access_level]
    /// - [UserMeta::subscriptions]
    /// - [UserMeta::created_at]
    ///
    /// That means access and update timestamps are not included in the comparison.
    fn eq(&self, other: &Self) -> bool {
        self.access_level == other.access_level
            && self.subscriptions == other.subscriptions
            && self.created_at == other.created_at
    }
}
