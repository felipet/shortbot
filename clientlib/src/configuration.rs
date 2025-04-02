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

//! `ClientLib` configuration module.
//!
//! # Description
//!
//! This module contains functions related to the configuration of the DB backend
//! in charge of the bot's client handling.

use bot_core::configuration::DatabaseSettings;
use secrecy::ExposeSecret;
use sqlx::mysql::{MySqlConnectOptions, MySqlSslMode};

pub fn build_db_conn_without_db(config: &DatabaseSettings) -> MySqlConnectOptions {
    MySqlConnectOptions::new()
        .host(&config.mariadb_host)
        .port(config.mariadb_port)
        .username(&config.mariadb_user)
        .password(&config.mariadb_password.expose_secret())
        .charset("utf8mb4")
        .ssl_mode(if config.mariadb_ssl_mode.unwrap_or_default() {
            MySqlSslMode::Required
        } else {
            MySqlSslMode::Preferred
        })
}

pub fn build_db_conn_with_db(config: &DatabaseSettings) -> MySqlConnectOptions {
    build_db_conn_without_db(config).database(&config.mariadb_dbname)
}
