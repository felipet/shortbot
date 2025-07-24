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

//! Module that includes all the logic related to the management of the user's metadata.
//!
//! # Description
//!
//! The `struct` [UserHandler] is the API for external modules that aim to request or modify data related to
//! users of the bot. That metadata is stored in an external cache, which is hidden from the rest of the modules
//! of Shortbot.

use crate::{
    DbError,
    configuration::ValkeySettings,
    errors::UserError,
    users::{BotAccess, Subscriptions, UserConfig, UserMeta},
};
use chrono::Utc;
use redis::{AsyncCommands, RedisError, aio::MultiplexedConnection};
use serde::Serialize;
use std::error::Error;
use teloxide::types::UserId;
use tracing::{debug, error, info, warn};

/// Handler for the management of the user's metadata.
#[derive(Clone)]
pub struct UserHandler {
    /// DB pool reference.
    db_client: redis::Client,
    db_settings: redis::AsyncConnectionConfig,
    hash_id: u64,
}

#[derive(Clone, Debug)]
enum ContentType {
    Meta,
    Config,
}

impl From<ContentType> for String {
    fn from(val: ContentType) -> Self {
        let str = match val {
            ContentType::Meta => "meta",
            ContentType::Config => "config",
        };

        str.to_owned()
    }
}

impl From<&ContentType> for String {
    fn from(val: &ContentType) -> Self {
        let str = match val {
            ContentType::Meta => "meta",
            ContentType::Config => "config",
        };

        str.to_owned()
    }
}

impl std::fmt::Display for ContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Into::<String>::into(self))
    }
}

impl UserHandler {
    /// Private method that retrieves a value from the dict and deserializes it.
    async fn get(
        &self,
        con: &mut MultiplexedConnection,
        user_id: &UserId,
        content_type: ContentType,
    ) -> Result<String, Box<dyn Error + Sync + Send>> {
        let json_data: String = con
            .hget(
                format!("shortbot:{}:{}", self.hash_id, user_id.0),
                content_type.to_string(),
            )
            .await?;

        Ok(json_data)
    }

    /// Private method that inserts a new value into the dict and serializes it.
    async fn set<T: Serialize>(
        &self,
        con: &mut MultiplexedConnection,
        user_id: &UserId,
        content_type: ContentType,
        meta: T,
    ) -> Result<(), Box<dyn Error + Sync + Send>> {
        let json_meta = serde_json::to_string(&meta)
            .map_err(|e| Box::new(UserError::SerialisationError(e.to_string())))?;

        let _: () = con
            .hset(
                format!("shortbot:{}:{}", self.hash_id, user_id.0),
                content_type.to_string(),
                json_meta,
            )
            .await?;

        Ok(())
    }

    /// The constructor builds a new Redis client from the global settings.
    pub async fn new(settings: &ValkeySettings) -> Result<Self, DbError> {
        Ok(UserHandler {
            db_client: redis::Client::open(format!(
                "redis://{}:{}/",
                settings.valkey_host.clone(),
                settings.valkey_port.clone(),
            ))
            .map_err(|e| DbError::UnknownValkey(e.to_string()))?,
            db_settings: settings.connection_config(),
            hash_id: settings.valkey_hash_id.unwrap_or(rand::random::<u64>()),
        })
    }

    /// Method that retrieves the access level of a Telegram user.
    ///
    /// # Description
    ///
    /// This method retrieves the level of access of an user, indicated by one of the variants of the `enum`
    /// [BotAccess]. When the access level of an unregistered user is requested, [BotAccess::Free] is returned.
    pub async fn access_level(
        &self,
        user_id: &UserId,
    ) -> Result<BotAccess, Box<dyn Error + Send + Sync>> {
        let mut con = self
            .db_client
            .get_multiplexed_async_connection_with_config(&self.db_settings)
            .await?;

        // Don't check if the user exists, send a raw get and check for the error type in case the user was
        // not registered.
        match self.get(&mut con, user_id, ContentType::Meta).await {
            Ok(json) => Ok(serde_json::from_str::<UserMeta>(&json)
                .map_err(|e| UserError::SerialisationError(e.to_string()))?
                .access_level),
            Err(e) => match e.downcast_ref::<RedisError>() {
                Some(redis_err) => {
                    if redis_err.kind() == redis::ErrorKind::TypeError {
                        warn!("Access level of non-registered user requested");
                        Ok(BotAccess::Free)
                    } else {
                        error!("Error detected while checking user's access level: {e}");
                        Err(e)
                    }
                }
                None => {
                    error!("Error detected while checking user's access level: {e}");
                    Err(e)
                }
            },
        }
    }

    /// Method that refreshes the last access time of the user.
    ///
    /// # Description
    ///
    /// This method is meant to be called anytime a handler of the bot is called from an user. On each call,
    /// the access time will get updated.
    ///
    /// If the method is called using a client ID which wasn't registered, an error [UserError::ClientNotRegistered]
    /// will be raised.
    pub async fn refresh_access(
        &self,
        user_id: &UserId,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut con = self
            .db_client
            .get_multiplexed_async_connection_with_config(&self.db_settings)
            .await?;

        // If this fails, the user wasn't registered, raise an error.
        match self.get(&mut con, user_id, ContentType::Meta).await {
            Ok(json) => {
                let mut meta: UserMeta = serde_json::from_str(&json)
                    .map_err(|e| UserError::SerialisationError(e.to_string()))?;
                meta.last_access = Utc::now();
                self.set(&mut con, user_id, ContentType::Meta, meta).await?;
                debug!("Access time refreshed for user: {user_id}");
                Ok(())
            }
            Err(e) => match e.downcast_ref::<RedisError>() {
                Some(redis_err) => {
                    if redis_err.kind() == redis::ErrorKind::TypeError {
                        error!("Attempt to refresh the access time of a non-registered user");
                        Err(Box::new(UserError::ClientNotRegistered))
                    } else {
                        Err(e)
                    }
                }
                None => Err(e),
            },
        }
    }

    /// Method that returns if a Telegram user is registered as a bot's user.
    pub async fn is_registered(
        &self,
        user_id: &UserId,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let mut con = self
            .db_client
            .get_multiplexed_async_connection_with_config(&self.db_settings)
            .await?;

        debug!("Checking if the user is registered");

        Ok(con
            .exists(format!("shortbot:{}:{}", self.hash_id, user_id.0))
            .await
            .map_err(|e| DbError::UnknownValkey(e.to_string()))?)
    }

    /// Method that registers an Telegram user as an user of the bot.
    pub async fn register_user(
        &self,
        user_id: &UserId,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        debug!("Proceed to register the user");
        let mut con = self
            .db_client
            .get_multiplexed_async_connection_with_config(&self.db_settings)
            .await?;

        let json_meta = serde_json::to_string(&UserMeta::new())
            .map_err(|e| Box::new(UserError::SerialisationError(e.to_string())))?;

        // Keep an eye on self.set
        let _: () = con
            .hset(
                format!("shortbot:{}:{}", self.hash_id, user_id.0),
                ContentType::Meta.to_string(),
                json_meta,
            )
            .await?;

        let json_config = serde_json::to_string(&UserConfig::default())
            .map_err(|e| Box::new(UserError::SerialisationError(e.to_string())))?;

        let _: () = con
            .hset(
                format!("shortbot:{}:{}", self.hash_id, user_id.0),
                ContentType::Config.to_string(),
                json_config,
            )
            .await?;

        info!("New user registered");

        Ok(())
    }

    /// Method that retrieves the subscriptions of the client.
    ///
    /// # Description
    ///
    /// If the user was not registered in the DB, an error [UserError::ClientNotRegistered] will be raised.
    pub async fn subscriptions(
        &self,
        user_id: &UserId,
    ) -> Result<Option<Subscriptions>, Box<dyn Error + Send + Sync>> {
        let mut con = self
            .db_client
            .get_multiplexed_async_connection_with_config(&self.db_settings)
            .await?;

        match self.get(&mut con, user_id, ContentType::Meta).await {
            Ok(json_meta) => Ok(serde_json::from_str::<UserMeta>(&json_meta)
                .map_err(|e| UserError::SerialisationError(e.to_string()))?
                .subscriptions),
            Err(e) => match e.downcast_ref::<RedisError>() {
                Some(redis_err) => {
                    if redis_err.kind() == redis::ErrorKind::TypeError {
                        error!("Attempt to get subscriptions of a non-registered user");
                        Err(Box::new(UserError::ClientNotRegistered))
                    } else {
                        Err(e)
                    }
                }
                None => Err(e),
            },
        }
    }

    /// Method that adds tickers to the subscription list of the user.
    ///
    /// # Description
    ///
    /// If the user was not registered in the DB, an error [UserError::ClientNotRegistered] will be raised.
    pub async fn add_subscriptions(
        &self,
        user_id: &UserId,
        subscriptions: Subscriptions,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut con = self
            .db_client
            .get_multiplexed_async_connection_with_config(&self.db_settings)
            .await?;

        match self.get(&mut con, user_id, ContentType::Meta).await {
            Ok(json_meta) => {
                let mut meta: UserMeta = serde_json::from_str(&json_meta)
                    .map_err(|e| UserError::SerialisationError(e.to_string()))?;

                info!("The user added new subscriptions: {subscriptions}");
                if meta.subscriptions.is_none() {
                    meta.subscriptions = Some(subscriptions);
                } else {
                    *meta.subscriptions.as_mut().unwrap() += subscriptions;
                }
                self.set(&mut con, user_id, ContentType::Meta, meta).await?;

                Ok(())
            }
            Err(e) => match e.downcast_ref::<RedisError>() {
                Some(redis_err) => {
                    if redis_err.kind() == redis::ErrorKind::TypeError {
                        error!("Attempt to add subscriptions of a non-registered user");
                        Err(Box::new(UserError::ClientNotRegistered))
                    } else {
                        Err(e)
                    }
                }
                None => Err(e),
            },
        }
    }

    /// Method that removes tickers from the subscription list of the client.
    ///
    /// # Description
    ///
    /// If the user was not registered in the DB, an error [UserError::ClientNotRegistered] will be raised.
    pub async fn remove_subscriptions(
        &self,
        user_id: &UserId,
        subscriptions: Subscriptions,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut con = self
            .db_client
            .get_multiplexed_async_connection_with_config(&self.db_settings)
            .await?;

        match self.get(&mut con, user_id, ContentType::Meta).await {
            Ok(json_meta) => {
                let mut meta: UserMeta = serde_json::from_str(&json_meta)
                    .map_err(|e| UserError::SerialisationError(e.to_string()))?;

                if meta.subscriptions.is_none() {
                    warn!("No subscriptions to remove");
                } else {
                    let subs = meta.subscriptions.as_mut().unwrap();
                    *subs -= subscriptions;

                    if subs.is_empty() {
                        meta.subscriptions = None;
                    }

                    self.set(&mut con, user_id, ContentType::Meta, meta).await?;
                }
                Ok(())
            }
            Err(e) => match e.downcast_ref::<RedisError>() {
                Some(redis_err) => {
                    if redis_err.kind() == redis::ErrorKind::TypeError {
                        error!("Attempt to remove subscriptions of a non-registered user");
                        Err(Box::new(UserError::ClientNotRegistered))
                    } else {
                        Err(e)
                    }
                }
                None => Err(e),
            },
        }
    }

    /// Method that modifies the access level of a client.
    ///
    /// # Description
    ///
    /// If the user was not registered in the DB, an error [UserError::ClientNotRegistered] will be raised.
    pub async fn modify_access_level(
        &self,
        user_id: &UserId,
        access: BotAccess,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut con = self
            .db_client
            .get_multiplexed_async_connection_with_config(&self.db_settings)
            .await?;

        match self.get(&mut con, user_id, ContentType::Meta).await {
            Ok(json_meta) => {
                let mut meta: UserMeta = serde_json::from_str(&json_meta)
                    .map_err(|e| UserError::SerialisationError(e.to_string()))?;
                meta.access_level = access;
                self.set(&mut con, user_id, ContentType::Meta, meta).await?;
                Ok(())
            }
            Err(e) => match e.downcast_ref::<RedisError>() {
                Some(redis_err) => {
                    if redis_err.kind() == redis::ErrorKind::TypeError {
                        error!("Attempt to modify access of a non-registered user");
                        Err(Box::new(UserError::ClientNotRegistered))
                    } else {
                        Err(e)
                    }
                }
                None => Err(e),
            },
        }
    }

    /// Method that retrieves the user's config.
    ///
    /// # Description
    ///
    /// If the user was not registered in the DB, an error [UserError::ClientNotRegistered] will be raised.
    pub async fn user_config(
        &self,
        user_id: &UserId,
    ) -> Result<UserConfig, Box<dyn Error + Send + Sync>> {
        let mut con = self
            .db_client
            .get_multiplexed_async_connection_with_config(&self.db_settings)
            .await?;

        match self.get(&mut con, user_id, ContentType::Config).await {
            Ok(json_config) => Ok(serde_json::from_str::<UserConfig>(&json_config)
                .map_err(|e| UserError::SerialisationError(e.to_string()))?),
            Err(e) => match e.downcast_ref::<RedisError>() {
                Some(redis_err) => {
                    if redis_err.kind() == redis::ErrorKind::TypeError {
                        warn!("Returning default config for non-registered user");
                        Ok(UserConfig::default())
                    } else {
                        Err(e)
                    }
                }
                None => Err(e),
            },
        }
    }

    /// Method that stores the user's config.
    ///
    /// # Description
    ///
    /// If the user was not registered in the DB, an error [UserError::ClientNotRegistered] will be raised.
    pub async fn modify_user_config(
        &self,
        user_id: &UserId,
        config: UserConfig,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut con = self
            .db_client
            .get_multiplexed_async_connection_with_config(&self.db_settings)
            .await?;

        let is_registered = self.is_registered(user_id).await?;

        if is_registered {
            let _: () = self
                .set(&mut con, user_id, ContentType::Config, config)
                .await?;
            debug!("User settings modified");
            Ok(())
        } else {
            error!("Can't modify the settings of a non-registered user");
            Err(Box::new(UserError::ClientNotRegistered))
        }
    }

    /// Method that returns a list of users of the bot
    ///
    /// # Description
    ///
    /// This method is meant to return a list of users that can be later used to send broadcast messages.
    /// If `ignore_settings` is `false`, the list will only contain the users whose settings enable
    /// broadcast messages. See [UserConfig::show_broadcast_msg].
    pub async fn list_users(
        &self,
        ignore_settings: bool,
    ) -> Result<Vec<u64>, Box<dyn Error + Send + Sync>> {
        let mut con = self
            .db_client
            .get_multiplexed_async_connection_with_config(&self.db_settings)
            .await?;

        let raw_keys: Vec<String> = con.keys(format!("shortbot:{}:*", self.hash_id)).await?;

        let keys: Vec<u64> = if ignore_settings {
            raw_keys
                .into_iter()
                .map(|k| k.split(':').next_back().unwrap().to_owned())
                .map(|k| k.parse::<u64>().unwrap())
                .collect()
        } else {
            let mut keys = Vec::new();

            for key in raw_keys.iter() {
                let k = key.split(":").last().unwrap().parse::<u64>().unwrap();
                let config: UserConfig = serde_json::from_str(
                    &self.get(&mut con, &UserId(k), ContentType::Config).await?,
                )?;

                if config.show_broadcast_msg {
                    keys.push(k);
                }
            }

            keys
        };

        debug!("List of existing users: {keys:?}");

        Ok(keys)
    }
}

impl Drop for UserHandler {
    fn drop(&mut self) {
        let mut con = match self.db_client.get_connection() {
            Ok(con) => con,
            Err(e) => {
                error!("Failed to get a connection to Valkey server: {e}");
                return;
            }
        };

        info!("Sending BGSAVE command to Valkey server to force saving the cache");
        match redis::cmd("BGSAVE").exec(&mut con) {
            Ok(_) => info!("User's cache successfully saved"),
            Err(e) => error!("Failed to save user's cache content: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{configuration::ValkeySettings, users::Subscriptions};
    use chrono::Duration;
    use once_cell::sync::Lazy;
    use pretty_assertions::assert_eq;
    use rand::random;
    use redis::AsyncCommands;
    use rstest::*;
    use tracing::{Level, level_filters::LevelFilter};
    use tracing_subscriber::{Layer, filter::Targets, fmt, prelude::*};

    static TRACING: Lazy<()> = Lazy::new(|| {
        if std::env::var("TEST_LOG").is_ok() {
            let level =
                std::env::var("TEST_LOG").expect("Failed to read the content of TEST_LOG var");

            let (tracing_level, tracing_levelfilter) = match level.as_str() {
                "info" => (Level::INFO, LevelFilter::INFO),
                "debug" => (Level::DEBUG, LevelFilter::DEBUG),
                "warn" => (Level::WARN, LevelFilter::WARN),
                "error" => (Level::ERROR, LevelFilter::ERROR),
                &_ => (Level::TRACE, LevelFilter::TRACE),
            };

            tracing_subscriber::registry()
                .with(
                    fmt::layer()
                        .with_ansi(false)
                        .with_target(true)
                        .with_filter(tracing_levelfilter),
                )
                .with(Targets::new().with_target("shortbot", tracing_level))
                .init();
        }
    });

    #[fixture]
    async fn user_handler_fixture() -> UserHandler {
        let settings = ValkeySettings {
            valkey_host: String::from("127.0.0.1"),
            valkey_port: 6379,
            valkey_conn_timeout: None,
            valkey_resp_timeout: None,
            // Use a random number
            valkey_hash_id: None,
        };

        UserHandler::new(&settings)
            .await
            .expect("Failed to instance a new UserHandler")
    }

    /// TC: Insert a new user into the dict.
    ///
    /// # Description
    ///
    /// ## Pre
    ///
    /// - The dict is empty.
    ///
    /// ## Inputs
    ///
    /// - A random user ID.
    ///
    /// ## TC
    ///
    /// This TC inserts metadata for a new user.
    ///
    /// ## Result
    ///
    /// The key of the UserId and the creating time must match the expectation.
    #[rstest]
    #[awt]
    #[tokio::test]
    async fn insert_user(#[future] user_handler_fixture: UserHandler) {
        Lazy::force(&TRACING);

        let user_id = UserId { 0: random::<u64>() };
        let settings = ValkeySettings {
            valkey_host: String::from("127.0.0.1"),
            valkey_port: 6379,
            valkey_conn_timeout: None,
            valkey_resp_timeout: None,
            valkey_hash_id: None,
        };

        let now = Utc::now();

        user_handler_fixture
            .register_user(&user_id)
            .await
            .expect("Failed to register a new user");

        let mut con = user_handler_fixture
            .db_client
            .get_multiplexed_async_connection_with_config(&settings.connection_config())
            .await
            .expect("Failed to open a new connection to Valkey");

        let stored_meta: String = con
            .hget(
                format!("shortbot:{}:{}", user_handler_fixture.hash_id, user_id.0),
                ContentType::Meta.to_string(),
            )
            .await
            .expect("Failed to retrieve the fresh user");
        let stored_meta: UserMeta =
            serde_json::from_str(&stored_meta).expect("Failed to deserialise");

        assert!(stored_meta.created_at - now < Duration::seconds(1));

        let stored_config: String = con
            .hget(
                format!("shortbot:{}:{}", user_handler_fixture.hash_id, user_id.0),
                ContentType::Config.to_string(),
            )
            .await
            .expect("Failed to retrieve the fresh user");
        let stored_config: UserConfig =
            serde_json::from_str(&stored_config).expect("Failed to deserialise");

        assert_eq!(stored_config, UserConfig::default());
    }

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
    #[rstest]
    #[awt]
    #[tokio::test]
    async fn add(#[future] user_handler_fixture: UserHandler) {
        Lazy::force(&TRACING);

        let user_id = UserId { 0: random::<u64>() };
        let settings = ValkeySettings {
            valkey_host: String::from("127.0.0.1"),
            valkey_port: 6379,
            valkey_conn_timeout: None,
            valkey_resp_timeout: None,
            valkey_hash_id: None,
        };

        user_handler_fixture
            .register_user(&user_id)
            .await
            .expect("Failed to register a new user");

        // First: let's insert a new subscription.
        let test_subscriptions = Subscriptions::try_from(["SAN"].as_ref())
            .expect("Failed to create a subscriptions object");
        user_handler_fixture
            .add_subscriptions(&user_id, test_subscriptions.clone())
            .await
            .expect("Failed to add new subscriptions");

        let mut con = user_handler_fixture
            .db_client
            .get_multiplexed_async_connection_with_config(&settings.connection_config())
            .await
            .expect("Failed to open a new connection to Valkey");

        let stored_meta: String = con
            .hget(
                format!("shortbot:{}:{}", user_handler_fixture.hash_id, user_id.0),
                ContentType::Meta.to_string(),
            )
            .await
            .expect("Failed to retrieve the fresh user");
        let stored_meta: UserMeta =
            serde_json::from_str(&stored_meta).expect("Failed to deserialise");

        assert_eq!(stored_meta.subscriptions, Some(test_subscriptions.clone()));

        // Second: let's try to insert the same subscription.
        user_handler_fixture
            .add_subscriptions(&user_id, test_subscriptions.clone())
            .await
            .expect("Failed to add new subscriptions");

        let stored_meta: String = con
            .hget(
                format!("shortbot:{}:{}", user_handler_fixture.hash_id, user_id.0),
                ContentType::Meta.to_string(),
            )
            .await
            .expect("Failed to retrieve the fresh user");
        let stored_meta: UserMeta =
            serde_json::from_str(&stored_meta).expect("Failed to deserialise");
        assert_eq!(stored_meta.subscriptions, Some(test_subscriptions.clone()));

        // Third: let's insert an array of subscriptions this time.
        let mut test_subscriptions = Subscriptions::try_from(["BBVA", "SAB"].as_ref())
            .expect("Failed to create a subscriptions object");

        user_handler_fixture
            .add_subscriptions(&user_id, test_subscriptions.clone())
            .await
            .expect("Failed to add new subscriptions");

        // SAN was inserted before in the dict.
        test_subscriptions.add_subscriptions(&["SAN"]);
        let stored_meta: String = con
            .hget(
                format!("shortbot:{}:{}", user_handler_fixture.hash_id, user_id.0),
                ContentType::Meta.to_string(),
            )
            .await
            .expect("Failed to retrieve the fresh user");
        let stored_meta: UserMeta =
            serde_json::from_str(&stored_meta).expect("Failed to deserialise");

        assert_eq!(stored_meta.subscriptions, Some(test_subscriptions));
    }

    /// TC: Remove a subscription for a registered client.
    ///
    /// # Description
    ///
    /// ## Pre
    ///
    /// - The cache includes a registered user.
    /// - The client has some subscriptions.
    /// - [UserHandler::add_subscriptions] works.
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
    /// The test subscriptions match the retrieved subscriptions from the dict.
    #[rstest]
    #[awt]
    #[tokio::test]
    async fn remove(#[future] user_handler_fixture: UserHandler) {
        Lazy::force(&TRACING);

        let user_id = UserId { 0: random::<u64>() };
        let settings = ValkeySettings {
            valkey_host: String::from("127.0.0.1"),
            valkey_port: 6379,
            valkey_conn_timeout: None,
            valkey_resp_timeout: None,
            valkey_hash_id: None,
        };

        user_handler_fixture
            .register_user(&user_id)
            .await
            .expect("Failed to register a new user");

        let mut con = user_handler_fixture
            .db_client
            .get_multiplexed_async_connection_with_config(&settings.connection_config())
            .await
            .expect("Failed to open a new connection to Valkey");

        // First: let's insert a new subscription.
        let mut test_subscriptions = Subscriptions::try_from(["SAN", "ENG", "REP", "IAG"].as_ref())
            .expect("Failed to create a subscriptions object");
        user_handler_fixture
            .add_subscriptions(&user_id, test_subscriptions.clone())
            .await
            .expect("Failed to add new subscriptions");

        // Time to attempt to remove a existing subscription.
        let to_remove = Subscriptions::try_from(["ENG"].as_ref())
            .expect("Failed to create a subscriptions object");
        test_subscriptions -= &to_remove;

        user_handler_fixture
            .remove_subscriptions(&user_id, to_remove.clone())
            .await
            .expect("Failed to remove subscriptions");

        let stored_meta: String = con
            .hget(
                format!("shortbot:{}:{}", user_handler_fixture.hash_id, user_id.0),
                ContentType::Meta.to_string(),
            )
            .await
            .expect("Failed to retrieve the fresh user");
        let stored_meta: UserMeta =
            serde_json::from_str(&stored_meta).expect("Failed to deserialise");
        assert_eq!(stored_meta.subscriptions, Some(test_subscriptions.clone()));

        // Let's try again but this time the subscription won't be there.
        user_handler_fixture
            .remove_subscriptions(&user_id, to_remove)
            .await
            .expect("Failed to remove subscriptions");

        let stored_meta: String = con
            .hget(
                format!("shortbot:{}:{}", user_handler_fixture.hash_id, user_id.0),
                ContentType::Meta.to_string(),
            )
            .await
            .expect("Failed to retrieve the fresh user");
        let stored_meta: UserMeta =
            serde_json::from_str(&stored_meta).expect("Failed to deserialise");
        assert_eq!(stored_meta.subscriptions, Some(test_subscriptions.clone()));

        // And multiple subscriptions at once.
        let to_remove = Subscriptions::try_from(["REP", "IAG"].as_ref())
            .expect("Failed to create a subscriptions object");
        test_subscriptions -= &to_remove;

        user_handler_fixture
            .remove_subscriptions(&user_id, to_remove.clone())
            .await
            .expect("Failed to remove subscriptions");

        let stored_meta: String = con
            .hget(
                format!("shortbot:{}:{}", user_handler_fixture.hash_id, user_id.0),
                ContentType::Meta.to_string(),
            )
            .await
            .expect("Failed to retrieve the fresh user");
        let stored_meta: UserMeta =
            serde_json::from_str(&stored_meta).expect("Failed to deserialise");
        assert_eq!(stored_meta.subscriptions, Some(test_subscriptions.clone()));
    }

    /// TC: Retrieve the subscriptions of an user.
    ///
    /// # Description
    ///
    /// ## Pre
    ///
    /// - The cache includes a registered user.
    /// - The user has some subscriptions.
    /// - [UserHandler::add_subscriptions] works.
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
    #[rstest]
    #[awt]
    #[tokio::test]
    async fn retrieve(#[future] user_handler_fixture: UserHandler) {
        Lazy::force(&TRACING);

        let user_id = UserId { 0: random::<u64>() };

        user_handler_fixture
            .register_user(&user_id)
            .await
            .expect("Failed to register a new user");

        let test_subscriptions = Subscriptions::try_from(["SAN", "REP"].as_ref())
            .expect("Failed to create a subscriptions object");
        user_handler_fixture
            .add_subscriptions(&user_id, test_subscriptions.clone())
            .await
            .expect("Failed to add new subscriptions");

        let subscriptions = user_handler_fixture
            .subscriptions(&user_id)
            .await
            .expect("Failed to retrieve the subscriptions of the client");

        assert_eq!(subscriptions, Some(test_subscriptions));

        // Now, let's wipe those subscriptions and check that we get a None.
        user_handler_fixture
            .remove_subscriptions(&user_id, subscriptions.unwrap())
            .await
            .expect("Failed to remove the existing subscriptions");

        let subscriptions = user_handler_fixture
            .subscriptions(&user_id)
            .await
            .expect("Failed to retrieve the subscriptions of the client");

        assert!(subscriptions.is_none());
    }

    /// TC: Get the access level of an unregistered user.
    ///
    /// # Description
    ///
    /// ## Pre
    ///
    /// - The cache is empty.
    /// - There are no users records in the DB.
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
    #[rstest]
    #[awt]
    #[tokio::test]
    async fn access_level_tc1(#[future] user_handler_fixture: UserHandler) {
        Lazy::force(&TRACING);

        let user_id = UserId { 0: random::<u64>() };

        let access_test = user_handler_fixture
            .access_level(&user_id)
            .await
            .expect("Error trying to get access level");
        assert_eq!(
            access_test,
            BotAccess::default(),
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
    #[rstest]
    #[awt]
    #[tokio::test]
    async fn access_level_tc2(#[future] user_handler_fixture: UserHandler) {
        Lazy::force(&TRACING);

        let access_level_table = vec![
            (UserId { 0: random::<u64>() }, BotAccess::Free),
            (UserId { 0: random::<u64>() }, BotAccess::Limited),
            (UserId { 0: random::<u64>() }, BotAccess::Unlimited),
            (UserId { 0: random::<u64>() }, BotAccess::Admin),
        ];

        // Modify the access level of the test clients according to the table.
        for (id, ba) in access_level_table.iter() {
            user_handler_fixture
                .register_user(id)
                .await
                .expect("Failed to register client");
            user_handler_fixture
                .modify_access_level(id, *ba)
                .await
                .expect("Failed to modify access");
        }

        // Test
        for (id, access) in access_level_table.iter() {
            assert_eq!(
                *access,
                user_handler_fixture
                    .access_level(id)
                    .await
                    .expect("Error trying to get access level")
            );
        }
    }

    #[rstest]
    #[awt]
    #[tokio::test]
    async fn is_registered(#[future] user_handler_fixture: UserHandler) {
        Lazy::force(&TRACING);

        let user_id = UserId { 0: random::<u64>() };

        assert!(
            !user_handler_fixture
                .is_registered(&user_id)
                .await
                .expect("Failed to check if the user was registered"),
            "False positive"
        );

        user_handler_fixture
            .register_user(&user_id)
            .await
            .expect("Failed to register test user");

        assert!(
            user_handler_fixture
                .is_registered(&user_id)
                .await
                .expect("Failed to check if the user was registered"),
            "Expected the user to be registered"
        );
    }

    #[rstest]
    #[awt]
    #[tokio::test]
    async fn handle_configuration(#[future] user_handler_fixture: UserHandler) {
        Lazy::force(&TRACING);

        let user_id = UserId { 0: random::<u64>() };

        user_handler_fixture
            .register_user(&user_id)
            .await
            .expect("Failed to register a new user");

        let default_config = user_handler_fixture
            .user_config(&user_id)
            .await
            .expect("Failed to retrive the user's config");

        assert_eq!(default_config, UserConfig::default());

        // Now, let's modify the settings and check it.
        let mut mod_config = UserConfig::default();
        mod_config.prefer_tickers = false;
        mod_config.show_broadcast_msg = false;

        user_handler_fixture
            .modify_user_config(&user_id, mod_config.clone())
            .await
            .expect("Failed to modify user's config");

        let read_config = user_handler_fixture
            .user_config(&user_id)
            .await
            .expect("Failed to retrive the user's config");

        assert_eq!(mod_config, read_config);
    }

    #[rstest]
    #[awt]
    #[tokio::test]
    async fn list_users(#[future] user_handler_fixture: UserHandler) {
        Lazy::force(&TRACING);

        let users_table = vec![
            (UserId { 0: random::<u64>() }, UserConfig::default()),
            (
                UserId { 0: random::<u64>() },
                UserConfig {
                    show_broadcast_msg: false,
                    ..Default::default()
                },
            ),
            (UserId { 0: random::<u64>() }, UserConfig::default()),
            (
                UserId { 0: random::<u64>() },
                UserConfig {
                    show_broadcast_msg: false,
                    ..Default::default()
                },
            ),
        ];

        // Modify the access level of the test clients according to the table.
        for (id, cfg) in users_table.iter() {
            user_handler_fixture
                .register_user(id)
                .await
                .expect("Failed to register client");
            user_handler_fixture
                .modify_user_config(id, cfg.clone())
                .await
                .expect("Failed to set config");
        }

        let list: Vec<UserId> = user_handler_fixture
            .list_users(false)
            .await
            .expect("Failed to list users")
            .into_iter()
            .map(|x| UserId(x))
            .collect();

        assert_eq!(list.len(), 2);
        assert!(list.contains(&users_table[0].0));
        assert!(list.contains(&users_table[2].0));

        let list: Vec<UserId> = user_handler_fixture
            .list_users(true)
            .await
            .expect("Failed to list users")
            .into_iter()
            .map(|x| UserId(x))
            .collect();

        assert_eq!(list.len(), 4);
        users_table
            .iter()
            .for_each(|x| assert!(list.contains(&x.0)));
    }
}
