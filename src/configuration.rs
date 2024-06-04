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
use secrecy::Secret;
use serde_derive::Deserialize;

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
    pub api_token: Secret<String>,
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
            .add_source(Environment::with_prefix("shortbot").separator("__"))
            .build()?;

        settings.try_deserialize()
    }
}
