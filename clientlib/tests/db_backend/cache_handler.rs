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
use clientlib::{BotAccess, ClientObjectsBuilder};
use random::Source;
use sqlx::Executor;
use std::str::FromStr;
use std::sync::Arc;
use teloxide::types::UserId;
use tokio::{
    sync::mpsc,
    time::{Duration, sleep},
};
use whirlwind::ShardMap;

#[tokio::test]
async fn dummy_start() {
    let app = test_setup().await;

    let (tx, rx) = mpsc::channel(10);
    let cache = Arc::new(ShardMap::new());

    let mut cache_handler = CacheHandler::new(app.pool.clone(), rx, cache);

    let handler_thread = tokio::spawn(async move {
        cache_handler.start().await;
    });

    sleep(Duration::from_millis(2)).await;
    tx.send("ping").await.unwrap();
    sleep(Duration::from_millis(2)).await;
    tx.send("stop").await.unwrap();
    handler_thread.await.unwrap();
}
