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

use configuration::{DatabaseSettings, Settings, build_db_conn_with_db, build_db_conn_without_db};
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, MySqlConnection, MySqlPool};
use tracing::{Level, subscriber::set_global_default};
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid;

static TRACING: Lazy<()> = Lazy::new(|| {
    if std::env::var("TEST_LOG").is_ok() {
        let level = std::env::var("TEST_LOG").expect("Failed to read the content of TEST_LOG var");
        let level = match level.as_str() {
            "info" => Some(Level::INFO),
            "debug" => Some(Level::DEBUG),
            "warn" => Some(Level::WARN),
            "error" => Some(Level::ERROR),
            &_ => None,
        };

        if level.is_some() {
            let subscriber = FmtSubscriber::builder()
                .with_max_level(level.unwrap())
                .finish();
            set_global_default(subscriber).expect("Failed to set subscriber.");
        }
    }
});

pub struct TestApp {
    pub pool: MySqlPool,
}

pub async fn test_setup() -> TestApp {
    Lazy::force(&TRACING);

    let configuration = {
        let mut cfg = Settings::new().expect("Failed to read configuration file.");
        cfg.database.mariadb_dbname = Uuid::new_v4().to_string();

        cfg
    };

    // Connect to the DB backend
    let db_pool = configure_database(&configuration.database).await;

    TestApp { pool: db_pool }
}

pub async fn configure_database(config: &DatabaseSettings) -> MySqlPool {
    // Connect to the testing DB without using a DB name, as we'll give a testing name.
    let mut conn = MySqlConnection::connect_with(&build_db_conn_without_db(config))
        .await
        .expect("Failed to connect to MariaDB.");

    conn.execute(format!(r#"CREATE DATABASE `{}`;"#, config.mariadb_dbname).as_str())
        .await
        .expect("Failed to create test DB.");

    // Migrate the DB
    let conn_pool = MySqlPool::connect_with(build_db_conn_with_db(&config))
        .await
        .expect("Failed to connect to MariaDB.");

    sqlx::migrate!("./migrations")
        .run(&conn_pool)
        .await
        .expect("Failed to migrate the testing DB.");

    conn_pool
}
