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
    users::{BotAccess, Subscriptions, UserMeta},
};
use chrono::Utc;
use redis::{AsyncCommands, aio::MultiplexedConnection};
use std::error::Error;
use teloxide::types::UserId;
use tracing::{debug, info, warn};

/// Handler for the management of the user's metadata.
#[derive(Clone)]
pub struct UserHandler {
    /// DB pool reference.
    db_client: redis::Client,
    db_settings: redis::AsyncConnectionConfig,
}

impl UserHandler {
    /// Private static method that retrieves a value from the dict and deserializes it.
    async fn get(
        con: &mut MultiplexedConnection,
        user_id: &UserId,
    ) -> Result<UserMeta, Box<dyn Error + Sync + Send>> {
        let json_meta: String = con.get(user_id.0).await?;
        let meta: UserMeta = serde_json::from_str(&json_meta)
            .map_err(|e| Box::new(UserError::SerialisationError(e.to_string())))?;

        Ok(meta)
    }

    /// Private static method that inserts a new value into the dict and serializes it.
    async fn set(
        con: &mut MultiplexedConnection,
        user_id: &UserId,
        meta: UserMeta,
    ) -> Result<(), Box<dyn Error + Sync + Send>> {
        let json_meta = serde_json::to_string(&meta)
            .map_err(|e| Box::new(UserError::SerialisationError(e.to_string())))?;

        let _: () = con.set(user_id.0, json_meta).await?;

        Ok(())
    }

    /// The constructor builds a new Redis client from the global settings.
    pub async fn new(settings: &ValkeySettings) -> Result<Self, DbError> {
        Ok(UserHandler {
            db_client: redis::Client::open(format!(
                "redis://{}:{}/",
                settings.valkey_host.clone(),
                settings.valkey_port.clone()
            ))
            .map_err(|e| DbError::UnknownValkey(e.to_string()))?,
            db_settings: settings.connection_config(),
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
        let is_registered = self.is_registered(user_id).await?;

        if is_registered {
            let meta: UserMeta = UserHandler::get(&mut con, user_id).await?;

            Ok(meta.access_level)
        } else {
            debug!("Access level requested for a non-registered user");
            Ok(BotAccess::Free)
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
    pub async fn refresh_access(
        &self,
        user_id: &UserId,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut con = self
            .db_client
            .get_multiplexed_async_connection_with_config(&self.db_settings)
            .await?;
        let is_registered = self.is_registered(user_id).await?;

        if is_registered {
            let mut meta: UserMeta = UserHandler::get(&mut con, user_id).await?;
            meta.last_access = Utc::now();
            UserHandler::set(&mut con, user_id, meta).await?;
        }

        debug!("Access time refreshed for user: {user_id}");

        Ok(())
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

        debug!("Checking if the user {user_id} is registered");

        Ok(con
            .exists(user_id.0)
            .await
            .map_err(|e| DbError::UnknownValkey(e.to_string()))?)
    }

    /// Method that registers an Telegram user as an user of the bot.
    pub async fn register_user(
        &self,
        user_id: &UserId,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut con = self
            .db_client
            .get_multiplexed_async_connection_with_config(&self.db_settings)
            .await?;

        let metadata = serde_json::to_string(&UserMeta::new())
            .map_err(|e| Box::new(UserError::SerialisationError(e.to_string())))?;

        let _: () = con
            .set(user_id.0, metadata)
            .await
            .map_err(|e| Box::new(DbError::UnknownValkey(e.to_string())))?;

        Ok(())
    }

    /// Method that retrieves the subscriptions of the client.
    pub async fn subscriptions(
        &self,
        user_id: &UserId,
    ) -> Result<Option<Subscriptions>, Box<dyn Error + Send + Sync>> {
        let mut con = self
            .db_client
            .get_multiplexed_async_connection_with_config(&self.db_settings)
            .await?;

        let is_registered = self.is_registered(user_id).await?;

        if is_registered {
            let meta = UserHandler::get(&mut con, user_id).await?;

            Ok(meta.subscriptions)
        } else {
            debug!("Attempt to retrieve subscriptions from a non-registered user");
            Ok(None)
        }
    }

    /// Method that adds tickers to the subscription list of the user.
    pub async fn add_subscriptions(
        &self,
        user_id: &UserId,
        subscriptions: Subscriptions,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut con = self
            .db_client
            .get_multiplexed_async_connection_with_config(&self.db_settings)
            .await?;

        let is_registered = self.is_registered(user_id).await?;

        if is_registered {
            let mut meta = UserHandler::get(&mut con, user_id).await?;

            if meta.subscriptions.is_none() {
                meta.subscriptions = Some(subscriptions);
            } else {
                *meta.subscriptions.as_mut().unwrap() += subscriptions;
            }
            UserHandler::set(&mut con, user_id, meta).await?;
            info!("The user added new subscriptions");
        } else {
            warn!("The user must register before adding subscriptions");
        }

        Ok(())
    }

    /// Method that removes tickers from the subscription list of the client.
    pub async fn remove_subscriptions(
        &self,
        user_id: &UserId,
        subscriptions: Subscriptions,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut con = self
            .db_client
            .get_multiplexed_async_connection_with_config(&self.db_settings)
            .await?;

        let is_registered = self.is_registered(user_id).await?;

        if is_registered {
            let mut meta = UserHandler::get(&mut con, user_id).await?;

            if meta.subscriptions.is_none() {
                warn!("Attempt to remove subscriptions from a non-registered user");
            } else {
                let subs = meta.subscriptions.as_mut().unwrap();
                *subs -= subscriptions;

                if subs.is_empty() {
                    meta.subscriptions = None;
                }

                UserHandler::set(&mut con, user_id, meta).await?;
            }
        }

        Ok(())
    }

    /// Method that modifies the access level of a client.
    pub async fn modify_access_level(
        &self,
        user_id: &UserId,
        access: BotAccess,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut con = self
            .db_client
            .get_multiplexed_async_connection_with_config(&self.db_settings)
            .await?;

        let is_registered = self.is_registered(user_id).await?;

        if is_registered {
            let mut meta = UserHandler::get(&mut con, user_id).await?;
            meta.access_level = access;
            UserHandler::set(&mut con, user_id, meta).await?;
        } else {
            warn!("Attempt to modify access level of a non-registered user");
        }

        Ok(())
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

        let stored_meta: UserMeta = con
            .get(user_id.0)
            .await
            .expect("Failed to retrieve the fresh user");

        assert!(stored_meta.created_at - now < Duration::seconds(1));
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

        let stored_meta: UserMeta = con
            .get(user_id.0)
            .await
            .expect("Failed to retrieve the fresh user");

        assert_eq!(stored_meta.subscriptions, Some(test_subscriptions.clone()));

        // Second: let's try to insert the same subscription.
        user_handler_fixture
            .add_subscriptions(&user_id, test_subscriptions.clone())
            .await
            .expect("Failed to add new subscriptions");

        let stored_meta: UserMeta = con
            .get(user_id.0)
            .await
            .expect("Failed to retrieve the fresh user");
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
        let stored_meta: UserMeta = con
            .get(user_id.0)
            .await
            .expect("Failed to retrieve the fresh user");

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

        let stored_meta: UserMeta = con
            .get(user_id.0)
            .await
            .expect("Failed to retrieve the fresh user");
        assert_eq!(stored_meta.subscriptions, Some(test_subscriptions.clone()));

        // Let's try again but this time the subscription won't be there.
        user_handler_fixture
            .remove_subscriptions(&user_id, to_remove)
            .await
            .expect("Failed to remove subscriptions");

        let stored_meta: UserMeta = con
            .get(user_id.0)
            .await
            .expect("Failed to retrieve the fresh user");
        assert_eq!(stored_meta.subscriptions, Some(test_subscriptions.clone()));

        // And multiple subscriptions at once.
        let to_remove = Subscriptions::try_from(["REP", "IAG"].as_ref())
            .expect("Failed to create a subscriptions object");
        test_subscriptions -= &to_remove;

        user_handler_fixture
            .remove_subscriptions(&user_id, to_remove.clone())
            .await
            .expect("Failed to remove subscriptions");

        let stored_meta: UserMeta = con
            .get(user_id.0)
            .await
            .expect("Failed to retrieve the fresh user");
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
}
