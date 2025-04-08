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

//! `ClientLib` library crate.
//!
//! # Description
//!
//! This crate splits all the logic that relies on the MariaDB backend. The main purpose of this separation
//! is to enable SQLx to properly analyze and build the queries of the application. The bot features
//! several DB backends, which is not supported by SQLx as of today.
//!
//! The most straightforward workaround is to split the code into several crates, each one of them connects to
//! a specific DB backend. This way, SQLx can analyze the code and build the queries properly.
//!
//! All the code related to handling client's preferences, subscriptions or any other information related to them
//! is included in this crate as it relies on the MariaDB backend.
//!
//! ## How To Develop This Library
//!
//! In order to build successfully all the code of the application, the following procedure must be followed:
//!
//! For each crate of the workspace:
//!
//! 1. Set up the environment variables for connecting to the DB backend, either via `export DATABASE_URL` or using
//!    `.env` files.
//! 2. Build the crate using `cargo build`.
//! 3. Run `cargo sqlx prepare` to generate the SQLx prepared queries.
//!
//! Remember to commit those files to the repository.
//!
//! After that, the whole workspace can be built using `cargo build`, but we need to run SQLx in offline mode:
//! `export SQLX_OFFLINE=true`.

use sqlx::{Executor, MySqlPool};
use std::str::FromStr;

pub mod configuration;

/// This enum represents the access level of a bot client.
///
/// # Description
///
/// The access level is used to determine the level of access to the bot's features for each client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BotAccess {
    Free,
    Limited,
    Unlimited,
    Admin,
}

impl FromStr for BotAccess {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "free" => Ok(BotAccess::Free),
            "limited" => Ok(BotAccess::Limited),
            "unlimited" => Ok(BotAccess::Unlimited),
            "admin" => Ok(BotAccess::Admin),
            _ => Err("Invalid BotAccess type"),
        }
    }
}

impl std::fmt::Display for BotAccess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BotAccess::Free => write!(f, "free"),
            BotAccess::Limited => write!(f, "limited"),
            BotAccess::Unlimited => write!(f, "unlimited"),
            BotAccess::Admin => write!(f, "admin"),
        }
    }
}

pub async fn register_client(
    pool: &MySqlPool,
    client_id: i64,
    auto: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO BotClient VALUES (?, ?, ?, NULL, CURRENT_TIMESTAMP(), NULL)",
        client_id,
        auto,
        BotAccess::Free.to_string(),
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_access(pool: &MySqlPool, client_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE BotClient SET last_access=CURRENT_TIMESTAMP() WHERE id = ?",
        client_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_access_level(pool: &MySqlPool, client_id: i64) -> Result<BotAccess, sqlx::Error> {
    let row = sqlx::query!("SELECT access FROM BotClient WHERE id = ?", client_id)
        .fetch_optional(pool)
        .await?;

    match row {
        Some(row) => Ok(BotAccess::from_str(&row.access).unwrap_or(BotAccess::Free)),
        None => Ok(BotAccess::Free),
    }
}

pub async fn modify_access_level(
    pool: &MySqlPool,
    client_id: i64,
    access_level: BotAccess,
) -> Result<(), sqlx::Error> {
    pool.execute(sqlx::query!(
        r#"UPDATE BotClient SET access = ? WHERE id = ?"#,
        access_level.to_string(),
        client_id
    ))
    .await?;

    Ok(())
}

pub async fn mark_as_registered(pool: &MySqlPool, client_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE BotClient SET registered = 1 WHERE id = ?",
        client_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn mark_as_unregistered(pool: &MySqlPool, client_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE BotClient SET registered = 0 WHERE id = ?",
        client_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn is_registered(pool: &MySqlPool, client_id: i64) -> Result<bool, sqlx::Error> {
    let row = sqlx::query!("SELECT registered FROM BotClient WHERE id = ?", client_id)
        .fetch_one(pool)
        .await?;

    Ok(row.registered != 0)
}

pub async fn get_subcriptions(
    pool: &MySqlPool,
    client_id: i64,
) -> Result<Vec<String>, sqlx::Error> {
    let row = sqlx::query!(
        "SELECT subscriptions FROM BotClient WHERE id = ?",
        client_id
    )
    .fetch_one(pool)
    .await?;

    match row.subscriptions {
        Some(tickers) => Ok(parse_tickers(&tickers)),
        None => Ok(Vec::new()),
    }
}

pub async fn add_subscription(
    pool: &MySqlPool,
    client_id: i64,
    ticker: &str,
) -> Result<(), sqlx::Error> {
    let mut tickers = get_subcriptions(pool, client_id).await?;
    if !tickers.contains(&ticker.to_string()) {
        tickers.push(ticker.to_string());
    }

    sqlx::query!(
        "UPDATE BotClient SET subscriptions = ? WHERE id = ?",
        format_tickers(&tickers),
        client_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn remove_subscription(
    pool: &MySqlPool,
    client_id: i64,
    ticker: &str,
) -> Result<(), sqlx::Error> {
    let mut tickers = get_subcriptions(pool, client_id).await?;
    tickers.retain(|x| x != ticker);

    sqlx::query!(
        "UPDATE BotClient SET subscriptions = ? WHERE id = ?",
        format_tickers(&tickers),
        client_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

fn parse_tickers(tickers: &str) -> Vec<String> {
    tickers.split(';').map(|x| x.to_string()).collect()
}

fn format_tickers(tickers: &[String]) -> String {
    tickers.join(";")
}
