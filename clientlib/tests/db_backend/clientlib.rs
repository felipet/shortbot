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

// use crate::helpers::test_setup;
// use clientlib::{BotAccess, ClientObjectsBuilder};
// use random::Source;
// use teloxide::types::UserId;
// use tokio::time::{Duration, sleep};

// #[tokio::test]
// async fn access_update() {
//     let app = test_setup().await;

//     let mut source = random::default(42);
//     let client_id = UserId {
//         0: source.read::<u64>(),
//     };
//     let access_level = BotAccess::Free;
//     let handler = ClientHandler::new(app.pool.clone(), 4, chrono::TimeDelta::days(1));

//     // Seed a client
//     app.pool
//         .execute(sqlx::query!(
//             r#"INSERT INTO BotClient (id,registered,access,subscriptions,created_at)
//             VALUES (?, 0, ?, NULL, CURRENT_TIMESTAMP())"#,
//             client_id.0,
//             access_level.to_string(),
//         ))
//         .await
//         .expect("Failed to seed the DB with a client.");

//     // First test case, the initial access shall be NULL.
//     let access_time = sqlx::query!(
//         "SELECT last_access FROM BotClient WHERE id = ?",
//         client_id.0,
//     )
//     .fetch_one(&app.pool)
//     .await
//     .expect("Failed to query access time of the test client");

//     assert!(access_time.last_access.is_none());``

//     assert!(handler.db_update_access_time(&client_id).await.is_ok());

//     let access_time_t1 = sqlx::query!(
//         "SELECT last_access FROM BotClient WHERE id = ?",
//         client_id.0,
//     )
//     .fetch_one(&app.pool)
//     .await
//     .expect("Failed to query access time of the test client")
//     .last_access
//     .unwrap();

//     // Let a second happen
//     sleep(Duration::from_secs(1)).await;
//     assert!(handler.db_update_access_time(&client_id).await.is_ok());

//     let access_time_t2 = sqlx::query!(
//         "SELECT last_access FROM BotClient WHERE id = ?",
//         client_id.0,
//     )
//     .fetch_one(&app.pool)
//     .await
//     .expect("Failed to query access time of the test client")
//     .last_access
//     .unwrap();

//     assert!(access_time_t2 > access_time_t1);
// }

// #[tokio::test]
// async fn subscriptions() {
//     let app = test_setup().await;

//     let mut source = random::default(42);
//     let client_id = UserId {
//         0: source.read::<u64>(),
//     };
//     let access_level = BotAccess::Free;
//     let handler = ClientHandler::new(app.pool.clone(), 4, chrono::TimeDelta::days(1));

//     // Seed a client
//     app.pool
//         .execute(sqlx::query!(
//             r#"INSERT INTO BotClient (id,registered,access,subscriptions,created_at)
//             VALUES (?, 0, ?, NULL, CURRENT_TIMESTAMP())"#,
//             client_id.0,
//             access_level.to_string(),
//         ))
//         .await
//         .expect("Failed to seed the DB with a client.");

//     // No subscriptions yet
//     let subscriptions = match handler.db_subscriptions(&client_id).await {
//         Ok(s) => s,
//         Err(e) => panic!("Error trying to get subscriptions: {}", e),
//     };

//     assert!(subscriptions.is_empty(), "Subscriptions should be empty");

//     handler
//         .db_add_subscriptions(&["SAN"], &client_id)
//         .await
//         .expect("Failed to add a subscription");
//     let subscriptions = handler
//         .db_subscriptions(&client_id)
//         .await
//         .expect("Failed to get subscriptions");
//     assert_eq!(subscriptions.into_iter().collect::<Vec<_>>(), ["SAN"]);

//     handler
//         .db_add_subscriptions(&["BBVA"], &client_id)
//         .await
//         .expect("Failed to add a subscription");
//     let subscriptions = handler
//         .db_subscriptions(&client_id)
//         .await
//         .expect("Failed to get subscriptions");
//     let temp_v = subscriptions.into_iter().collect::<Vec<_>>();
//     assert!(temp_v == ["SAN", "BBVA"] || temp_v == ["BBVA", "SAN"]);

//     handler
//         .db_remove_subscriptions(&["BBVA"], &client_id)
//         .await
//         .expect("Failed to remove a subscription");
//     let subscriptions = handler
//         .db_subscriptions(&client_id)
//         .await
//         .expect("Failed to get subscriptions");
//     assert_eq!(subscriptions.into_iter().collect::<Vec<_>>(), ["SAN"]);

//     handler
//         .db_remove_subscriptions(&["SAB"], &client_id)
//         .await
//         .expect("Failed to remove a subscription");
//     let subscriptions = handler
//         .db_subscriptions(&client_id)
//         .await
//         .expect("Failed to get subscriptions");
//     assert_eq!(subscriptions.into_iter().collect::<Vec<_>>(), ["SAN"]);

//     handler
//         .db_add_subscriptions(&["SAN"], &client_id)
//         .await
//         .expect("Failed to add a subscription");
//     let subscriptions = handler
//         .db_subscriptions(&client_id)
//         .await
//         .expect("Failed to get subscriptions");
//     assert_eq!(subscriptions.into_iter().collect::<Vec<_>>(), ["SAN"]);
// }
