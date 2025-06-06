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

//! Module that contains definitions related to metadata for clients of the bot.

use crate::{BotAccess, Subscriptions};
use chrono::{DateTime, Utc};
use sqlx::{FromRow, Row, mysql::MySqlRow};
use std::str::FromStr;

/// Metadata of a bot client.
///
/// # Description
///
/// This `struct` represents a data object for a client of the bot. It contains
/// data that is stored in a DB, but also data that is only needed for the
/// internal use of the cache.
#[derive(Debug, Default, Clone, Eq)]
pub struct ClientMeta {
    /// Flag that identifies when a client was soft-registered or hard-registered.
    pub registered: bool,
    /// Identifies the level of access of the client. See [BotAccess].
    pub access_level: BotAccess,
    /// List of subscriptions of the client.
    pub subscriptions: Option<Subscriptions>,
    /// Timestamp of the last access of the client to the bot.
    pub last_access: Option<DateTime<Utc>>,
    /// Timestamp of the last cache update.
    pub last_update: Option<DateTime<Utc>>,
    /// Timestamp of the client register process.
    pub created_at: Option<DateTime<Utc>>,
}

impl FromRow<'_, MySqlRow> for ClientMeta {
    // TODO: proper handling of errors from try_get
    fn from_row(row: &'_ MySqlRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            registered: row.try_get::<i8, &str>("registered")? != 0,
            access_level: BotAccess::from_str(row.try_get("access")?).unwrap_or_default(),
            subscriptions: match row.try_get::<&str, &str>("subscriptions") {
                Ok(s) => Some(Subscriptions::try_from(s).unwrap_or_default()),
                Err(_) => None,
            },
            last_access: Some(
                row.try_get::<&str, &str>("last_access")?
                    .parse::<DateTime<Utc>>()
                    .unwrap(),
            ),
            last_update: None,
            created_at: Some(
                row.try_get::<&str, &str>("created_at")?
                    .parse::<DateTime<Utc>>()
                    .unwrap(),
            ),
        })
    }
}

impl PartialEq for ClientMeta {
    /// Compare two instances of [ClientMeta].
    ///
    /// # Description
    ///
    /// Two instances _are the same_ if all these members are the same on both sides:
    /// - [ClientMeta::registered]
    /// - [ClientMeta::access_level]
    /// - [ClientMeta::subscriptions]
    /// - [ClientMeta::created_at]
    ///
    /// That means access and update timestamps are not included in the comparison.
    fn eq(&self, other: &Self) -> bool {
        self.registered == other.registered
            && self.access_level == other.access_level
            && self.subscriptions == other.subscriptions
            && self.created_at == other.created_at
    }
}
