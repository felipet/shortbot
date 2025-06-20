// Copyright 2024-2025 Felipe Torres González
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

//! ShortBot configuration module
//!
//! # Description
//!
//! This module includes all the definitions for the app's settings and the
//! objects that automate reading the configuration from files or environment
//! variables and parsing them to Rust's native types.
//!
//! Some settings must be overrided by environment variables, for example, the
//! API token for the Telegram Bot client. All the environment variables that
//! are meant to be used within this module shall use the prefix _SHORTBOT_.

use config::{Config, ConfigError, Environment, File};
use secrecy::{ExposeSecret, SecretString};
use serde_derive::Deserialize;
use sqlx::mysql::{MySqlConnectOptions, MySqlSslMode};
use sqlx::postgres::{PgConnectOptions, PgSslMode};

/// Name of the directory in which configuration files will be stored.
const CONF_DIR: &str = "config";

/// Main settings `struct`.
#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Settings {
    /// Level for the tracing crate.
    pub tracing_level: String,
    /// Application specific settings.
    pub application: ApplicationSettings,
    /// Data folder path.
    pub data_path: String,
    /// Database backend settings.
    pub database: DatabaseSettings,
    /// Clientlib settings.
    pub clientlib: ClientlibSettings,
}

/// Settings of the ShortBot application.
///
/// # Description
///
/// - [ApplicationSettings::api_token]: Telegram BOT API token. Override the value
///   of the YML file using an environment variable: `export SHORTBOT__APPLICATION__API_KEY="key"`.
#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct ApplicationSettings {
    pub api_token: SecretString,
    pub http_server_host: String,
    pub http_server_port: u16,
    pub webhook_url: String,
    pub webhook_path: String,
    pub webhook_token: SecretString,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        // Build the full path of the configuration directory.
        let mut base_path =
            std::env::current_dir().expect("Failed to determine the current directory.");
        // Needed due to the workspace organisation of the project.
        base_path.push("../");
        let cfg_dir = base_path.join(CONF_DIR);

        let settings = Config::builder()
            // Start of  by merging in the "default" configuration file.
            .add_source(File::from(cfg_dir.join("base")).required(true))
            .add_source(Environment::with_prefix("shortbot").separator("__"))
            .build()?;

        settings.try_deserialize()
    }
}

/// Settings for the database backend.
#[derive(Debug, Deserialize)]
pub struct DatabaseSettings {
    pub questdb_host: String,
    pub questdb_port: u16,
    pub questdb_user: String,
    pub questdb_password: SecretString,
    pub mariadb_host: String,
    pub mariadb_port: u16,
    pub mariadb_user: String,
    pub mariadb_password: SecretString,
    pub mariadb_dbname: String,
    pub mariadb_ssl_mode: Option<bool>,
}

/// Settings for clientlib.
#[derive(Debug, Deserialize)]
pub struct ClientlibSettings {
    pub enable_cache: bool,
    pub cache_queue_size: u16,
    pub cache_shards: u16,
}

impl DatabaseSettings {
    pub fn questdb_connection(&self) -> PgConnectOptions {
        PgConnectOptions::new()
            .host(&self.questdb_host)
            .username(&self.questdb_user)
            .password(self.questdb_password.expose_secret())
            .port(self.questdb_port)
            .ssl_mode(PgSslMode::Prefer)
    }
}

pub fn build_db_conn_without_db(config: &DatabaseSettings) -> MySqlConnectOptions {
    MySqlConnectOptions::new()
        .host(&config.mariadb_host)
        .port(config.mariadb_port)
        .username(&config.mariadb_user)
        .password(config.mariadb_password.expose_secret())
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
