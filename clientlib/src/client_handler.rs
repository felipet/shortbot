use crate::{BotAccess, ClientError, Subscriptions};
use chrono::{DateTime, TimeDelta, Utc};
use sqlx::{Executor, MySqlPool};
use std::str::FromStr;
use teloxide::types::UserId;
use tracing::warn;
use whirlwind::ShardMap;

pub struct ClientHandler {
    db_conn: MySqlPool,
    cache: ShardMap<UserId, ClientMeta>,
    client_list: Vec<UserId>,
    cache_expiricy: TimeDelta,
}

pub struct ClientMeta {
    pub registered: bool,
    pub access_level: BotAccess,
    pub subscriptions: Subscriptions,
    pub last_access: DateTime<Utc>,
    pub last_update: DateTime<Utc>,
}

impl ClientHandler {
    pub fn new(db_conn: MySqlPool, shards: usize, cache_expiricy: TimeDelta) -> Self {
        let cache = ShardMap::with_shards(shards);

        ClientHandler {
            db_conn,
            cache,
            client_list: Vec::new(),
            cache_expiricy,
        }
    }

    pub async fn access_level(&self, client_id: &UserId) -> Result<BotAccess, ClientError> {
        Ok(BotAccess::Free)
    }

    pub async fn db_access_level(&self, client_id: &UserId) -> Result<BotAccess, ClientError> {
        let row = sqlx::query!("SELECT access FROM BotClient WHERE id = ?", client_id.0)
            .fetch_optional(&self.db_conn)
            .await?;

        match row {
            Some(row) => Ok(BotAccess::from_str(&row.access).unwrap_or(BotAccess::Free)),
            None => Ok(BotAccess::Free),
        }
    }

    pub async fn db_is_registered(&self, client_id: &UserId) -> Result<bool, ClientError> {
        let row = sqlx::query!("SELECT registered FROM BotClient WHERE id = ?", client_id.0)
            .fetch_optional(&self.db_conn)
            .await?;

        match row {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    pub async fn db_register_client(
        &self,
        client_id: &UserId,
        auto_register: bool,
    ) -> Result<(), ClientError> {
        sqlx::query!(
            "INSERT INTO BotClient VALUES (?, ?, ?, NULL, CURRENT_TIMESTAMP(), NULL)",
            client_id.0,
            auto_register,
            BotAccess::Free.to_string(),
        )
        .execute(&self.db_conn)
        .await?;

        Ok(())
    }

    pub async fn db_mark_as_registered(&self, client_id: &UserId) -> Result<(), ClientError> {
        sqlx::query!(
            "UPDATE BotClient SET registered = true WHERE id = ?",
            client_id.0
        )
        .execute(&self.db_conn)
        .await?;

        Ok(())
    }

    pub async fn db_modify_access_level(
        &self,
        client_id: &UserId,
        access_level: BotAccess,
    ) -> Result<(), ClientError> {
        sqlx::query!(
            "UPDATE BotClient SET access = ? WHERE id = ?",
            access_level.to_string(),
            client_id.0
        )
        .execute(&self.db_conn)
        .await?;

        Ok(())
    }

    pub async fn db_update_access_time(&self, client_id: &UserId) -> Result<(), ClientError> {
        sqlx::query!(
            "UPDATE BotClient SET last_access = CURRENT_TIMESTAMP() WHERE id = ?",
            client_id.0
        )
        .execute(&self.db_conn)
        .await?;

        Ok(())
    }

    pub async fn db_subscriptions(&self, client_id: &UserId) -> Result<Subscriptions, ClientError> {
        let row = sqlx::query!(
            "SELECT subscriptions FROM BotClient WHERE id = ?",
            client_id.0
        )
        .fetch_one(&self.db_conn)
        .await?;

        match row.subscriptions {
            Some(tickers) => Subscriptions::try_from(tickers),
            None => Ok(Subscriptions::default()),
        }
    }

    pub async fn db_add_subscriptions(
        &self,
        subscriptions: &[&str],
        client_id: &UserId,
    ) -> Result<Subscriptions, ClientError> {
        let mut tickers = self.db_subscriptions(client_id).await?;

        tickers.add_subscriptions(subscriptions);

        self.db_update_subscriptions(
            Into::<Vec<String>>::into(tickers.clone())
                .iter()
                .map(|e| e.as_str())
                .collect::<Vec<_>>()
                .as_slice(),
            client_id,
        )
        .await?;

        Ok(tickers)
    }

    pub async fn db_remove_subscriptions(
        &self,
        subscriptions: &[&str],
        client_id: &UserId,
    ) -> Result<Subscriptions, ClientError> {
        let mut tickers = self.db_subscriptions(client_id).await?;

        tickers.remove_subscriptions(subscriptions);

        self.db_update_subscriptions(
            Into::<Vec<String>>::into(tickers.clone())
                .iter()
                .map(|e| e.as_str())
                .collect::<Vec<_>>()
                .as_slice(),
            client_id,
        )
        .await?;

        Ok(tickers)
    }

    pub async fn db_update_subscriptions(
        &self,
        subscriptions: &[&str],
        client_id: &UserId,
    ) -> Result<(), ClientError> {
        sqlx::query!(
            "UPDATE BotClient SET subscriptions = ? WHERE id = ?",
            subscriptions.join(";"),
            client_id.0
        )
        .execute(&self.db_conn)
        .await?;

        Ok(())
    }
}
