use crate::{BotAccess, ClientDbHandler, ClientError, Subscriptions};
use sqlx::MySqlPool;
use std::str::FromStr;
use teloxide::types::UserId;

pub struct ClientHandler {
    db_conn: MySqlPool,
}

impl ClientDbHandler for ClientHandler {
    async fn access_level(&self, client_id: UserId) -> Result<BotAccess, ClientError> {
        let row = sqlx::query!("SELECT access FROM BotClient WHERE id = ?", client_id.0)
            .fetch_optional(&self.db_conn)
            .await?;

        match row {
            Some(row) => Ok(BotAccess::from_str(&row.access).unwrap_or(BotAccess::Free)),
            None => Ok(BotAccess::Free),
        }
    }

    async fn is_registered(&self, client_id: UserId) -> Result<bool, ClientError> {
        let row = sqlx::query!("SELECT registered FROM BotClient WHERE id = ?", client_id.0)
            .fetch_one(&self.db_conn)
            .await?;

        Ok(row.registered != 0)
    }

    async fn register_client(
        &self,
        client_id: UserId,
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

    async fn mark_as_registered(&self, client_id: UserId) -> Result<(), ClientError> {
        sqlx::query!(
            "UPDATE BotClient SET registered = true WHERE id = ?",
            client_id.0
        )
        .execute(&self.db_conn)
        .await?;

        Ok(())
    }

    async fn modify_access_level(
        &self,
        client_id: UserId,
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

    async fn update_access_time(&self, client_id: UserId) -> Result<(), ClientError> {
        sqlx::query!(
            "UPDATE BotClient SET last_access = CURRENT_TIMESTAMP() WHERE id = ?",
            client_id.0
        )
        .execute(&self.db_conn)
        .await?;

        Ok(())
    }

    async fn subscriptions(&self, client_id: UserId) -> Result<Subscriptions, ClientError> {
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

    async fn add_subscriptions(
        &self,
        subscriptions: &[&str],
        client_id: UserId,
    ) -> Result<Subscriptions, ClientError> {
        let mut tickers = self.subscriptions(client_id).await?;

        tickers.add_subscriptions(subscriptions);

        self.update_subscriptions(
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

    async fn remove_subscriptions(
        &self,
        subscriptions: &[&str],
        client_id: UserId,
    ) -> Result<Subscriptions, ClientError> {
        let mut tickers = self.subscriptions(client_id).await?;

        tickers.remove_subscriptions(subscriptions);

        self.update_subscriptions(
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
}

impl ClientHandler {
    pub fn new(db_conn: MySqlPool) -> Self {
        ClientHandler { db_conn }
    }

    async fn update_subscriptions(
        &self,
        subscriptions: &[&str],
        client_id: UserId,
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

// struct ClientMeta {
//     pub registered: bool,
//     pub manual_registered: bool,
// }

// pub struct ClientHandler {
//     registered_clients: HashMap<UserId, ClientMeta>
// }

// impl Default for ClientHandler {
//     fn default() -> Self {
//         Self {
//             ..Default::default()
//         }
//     }
// }
