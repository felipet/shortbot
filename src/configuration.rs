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
//! Some settings must be overridden by environment variables, for example, the
//! API token for the Telegram Bot client. All the environment variables that
//! are meant to be used within this module shall use the prefix _SHORTBOT_.

use config::{Config, ConfigError, Environment, File};
use redis::AsyncConnectionConfig;
use secrecy::{ExposeSecret, SecretString};
use serde_derive::Deserialize;
use sqlx::postgres::{PgConnectOptions, PgSslMode};
use std::time::Duration;

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
    /// Valkey backend to hold user's data.
    pub users_db: ValkeySettings,
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
        let base_path =
            std::env::current_dir().expect("Failed to determine the current directory.");
        let cfg_dir = base_path.join(CONF_DIR);

        let settings = Config::builder()
            // Start of  by merging in the "default" configuration file.
            .add_source(File::from(cfg_dir.join("base")).required(true))
            .add_source(File::from(cfg_dir.join("local")).required(false))
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

const VALKEY_CONN_TIMEOUT: u64 = 1;
const VALKEY_RESP_TIMEOUT: u64 = 1;

/// Settings for Valkey
#[derive(Debug, Deserialize)]
pub struct ValkeySettings {
    pub valkey_host: String,
    pub valkey_port: u16,
    pub valkey_conn_timeout: Option<u64>,
    pub valkey_resp_timeout: Option<u64>,
}

impl ValkeySettings {
    pub fn connection_config(&self) -> AsyncConnectionConfig {
        AsyncConnectionConfig::new()
            .set_connection_timeout(Duration::from_secs(
                self.valkey_conn_timeout.unwrap_or(VALKEY_CONN_TIMEOUT),
            ))
            .set_response_timeout(Duration::from_secs(
                self.valkey_resp_timeout.unwrap_or(VALKEY_RESP_TIMEOUT),
            ))
    }
}
