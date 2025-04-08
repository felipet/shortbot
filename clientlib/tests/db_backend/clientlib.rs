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

use std::str::FromStr;

use crate::helpers::test_setup;
use clientlib::{
    BotAccess, add_subscription, get_access_level, get_subcriptions, mark_as_registered,
    modify_access_level, register_client, remove_subscription, update_access,
};
use random::Source;
use sqlx::Executor;
use tokio::time::{Duration, sleep};

#[tokio::test]
async fn client_access_level() {
    let app = test_setup().await;
    let mut source = random::default(42);
    let client_id = source.read::<i64>();
    let access_level = BotAccess::Free;

    // Seed a client with free access
    app.pool
        .execute(sqlx::query!(
            r#"INSERT INTO BotClient (id,registered,access,subscriptions,created_at)
            VALUES (?, 0, ?, NULL, CURRENT_TIMESTAMP())"#,
            client_id,
            access_level.to_string(),
        ))
        .await
        .expect("Failed to seed the DB with a client.");

    let access_test = get_access_level(&app.pool, client_id)
        .await
        .expect("Error trying to get access level");
    assert_eq!(access_test, access_level, "Access level should be free");

    let access_level = BotAccess::Limited;
    assert!(
        modify_access_level(&app.pool, client_id, access_level.clone())
            .await
            .is_ok()
    );

    let access_test = get_access_level(&app.pool, client_id)
        .await
        .expect("Error trying to get access level");
    assert_eq!(access_test, access_level, "Access level should be limited");
}

#[tokio::test]
async fn access_update() {
    let app = test_setup().await;

    let mut source = random::default(42);
    let client_id = source.read::<i64>();
    let access_level = BotAccess::Free;

    // Seed a client
    app.pool
        .execute(sqlx::query!(
            r#"INSERT INTO BotClient (id,registered,access,subscriptions,created_at)
            VALUES (?, 0, ?, NULL, CURRENT_TIMESTAMP())"#,
            client_id,
            access_level.to_string(),
        ))
        .await
        .expect("Failed to seed the DB with a client.");

    // First test case, the initial access shall be NULL.
    let access_time = sqlx::query!("SELECT last_access FROM BotClient WHERE id = ?", client_id,)
        .fetch_one(&app.pool)
        .await
        .expect("Failed to query access time of the test client");

    assert!(access_time.last_access.is_none());

    assert!(update_access(&app.pool, client_id).await.is_ok());

    let access_time_t1 = sqlx::query!("SELECT last_access FROM BotClient WHERE id = ?", client_id,)
        .fetch_one(&app.pool)
        .await
        .expect("Failed to query access time of the test client")
        .last_access
        .unwrap();

    // Let a second happen
    sleep(Duration::from_secs(1)).await;
    assert!(update_access(&app.pool, client_id).await.is_ok());

    let access_time_t2 = sqlx::query!("SELECT last_access FROM BotClient WHERE id = ?", client_id,)
        .fetch_one(&app.pool)
        .await
        .expect("Failed to query access time of the test client")
        .last_access
        .unwrap();

    assert!(access_time_t2 > access_time_t1);
}

#[tokio::test]
async fn register() {
    let app = test_setup().await;

    let mut source = random::default(42);
    let client_id = source.read::<i64>();
    let auto_register = false;

    register_client(&app.pool, client_id, auto_register)
        .await
        .expect("Failed to register a new client");

    let test_client = sqlx::query!("SELECT * FROM BotClient WHERE id=?", client_id)
        .fetch_one(&app.pool)
        .await
        .expect("Failed to query a test client");

    assert_eq!(test_client.id, client_id);
    assert_eq!(
        BotAccess::from_str(&test_client.access).unwrap(),
        BotAccess::Free
    );
    assert!(test_client.created_at.is_some());
    assert!(
        test_client.last_access.is_none(),
        "Last access should be NULL"
    );
    let registered = if test_client.registered > 0 {
        true
    } else {
        false
    };
    assert_eq!(registered, auto_register);

    assert!(mark_as_registered(&app.pool, client_id).await.is_ok());

    let test_client = sqlx::query!("SELECT * FROM BotClient WHERE id=?", client_id)
        .fetch_one(&app.pool)
        .await
        .expect("Failed to query a test client");

    assert!(test_client.registered > 0, "Client should be registered");
}

#[tokio::test]
async fn subscriptions() {
    let app = test_setup().await;

    let mut source = random::default(42);
    let client_id = source.read::<i64>();
    let access_level = BotAccess::Free;

    // Seed a client
    app.pool
        .execute(sqlx::query!(
            r#"INSERT INTO BotClient (id,registered,access,subscriptions,created_at)
            VALUES (?, 0, ?, NULL, CURRENT_TIMESTAMP())"#,
            client_id,
            access_level.to_string(),
        ))
        .await
        .expect("Failed to seed the DB with a client.");

    // No subscriptions yet
    let subscriptions = match get_subcriptions(&app.pool, client_id).await {
        Ok(s) => s,
        Err(e) => panic!("Error trying to get subscriptions: {}", e),
    };

    assert!(subscriptions.is_empty(), "Subscriptions should be empty");

    add_subscription(&app.pool, client_id, "SAN")
        .await
        .expect("Failed to add a subscription");
    let subscriptions = get_subcriptions(&app.pool, client_id)
        .await
        .expect("Failed to get subscriptions");
    assert_eq!(subscriptions, ["SAN"]);

    add_subscription(&app.pool, client_id, "BBVA")
        .await
        .expect("Failed to add a subscription");
    let subscriptions = get_subcriptions(&app.pool, client_id)
        .await
        .expect("Failed to get subscriptions");
    assert_eq!(subscriptions, ["SAN", "BBVA"]);

    remove_subscription(&app.pool, client_id, "BBVA")
        .await
        .expect("Failed to remove a subscription");
    let subscriptions = get_subcriptions(&app.pool, client_id)
        .await
        .expect("Failed to get subscriptions");
    assert_eq!(subscriptions, ["SAN"]);

    remove_subscription(&app.pool, client_id, "SAB")
        .await
        .expect("Failed to remove a subscription");
    let subscriptions = get_subcriptions(&app.pool, client_id)
        .await
        .expect("Failed to get subscriptions");
    assert_eq!(subscriptions, ["SAN"]);

    add_subscription(&app.pool, client_id, "SAN")
        .await
        .expect("Failed to add a subscription");
    let subscriptions = get_subcriptions(&app.pool, client_id)
        .await
        .expect("Failed to get subscriptions");
    assert_eq!(subscriptions, ["SAN"]);
}
