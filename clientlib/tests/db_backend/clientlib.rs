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

use crate::helpers::test_setup;
use clientlib::{BotAccess, get_access_level, modify_access_level};
use random::Source;
use sqlx::Executor;

#[tokio::test]
async fn test_client_access() {
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
