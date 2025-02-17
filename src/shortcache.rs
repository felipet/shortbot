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

//! Module with the logic for the short positions cache.

use crate::{configuration::DatabaseSettings, errors::DbError};
use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone, Utc};
use data_harvest::domain::{AliveShortPositions, ShortPosition};
use finance_ibex::IbexCompany;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tracing::{debug, error, instrument, trace};
use uuid::Uuid;

pub struct ShortCache {
    db_pool: PgPool,
}

impl ShortCache {
    #[instrument(name = "Connect DB backend for the short positions", skip(settings))]
    pub async fn connect_backend(settings: &DatabaseSettings) -> Result<Self, DbError> {
        let db_pool = PgPoolOptions::new()
            .connect_with(settings.questdb_connection())
            .await
            .map_err(|e| {
                error!("{e}");
                DbError::UnknownQdb(e.to_string())
            })?;

        trace!("QuestDB database server succesfully connected");

        Ok(Self { db_pool })
    }

    pub async fn ibex35_listing(&self) -> Result<Vec<IbexCompany>, DbError> {
        let companies = sqlx::query_as!(IbexCompanyBd, "SELECT * FROM ibex35_listing",)
            .fetch_all(&self.db_pool)
            .await
            .map_err(|e| DbError::Unknown(e.to_string()))?;

        debug!("Obtained {} companies from the DB", companies.len());

        let companies = match companies.iter().map(IbexCompany::try_from).collect() {
            Ok(c) => c,
            Err(e) => return Err(e),
        };

        Ok(companies)
    }

    #[instrument(name = "Retrive short positions", skip(self))]
    pub async fn short_position(&self, ticker: &str) -> Result<AliveShortPositions, DbError> {
        let positions = sqlx::query_as!(
            ShortPositionBd,
            r#"
            SELECT alive_positions.id, owner, weight, open_date, ticker
            FROM alive_positions INNER JOIN ibex35_short_historic on alive_positions.id = ibex35_short_historic.id
            WHERE ibex35_short_historic.ticker = $1
            "#,
            ticker,
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| DbError::Unknown(e.to_string()))?;

        // let positions = match positions.iter().map(ShortPosition::try_from).collect() {
        //     Ok(v) => v,
        //     Err(e) => Err(e),
        // };

        let mut shorts = Vec::new();

        for position in positions {
            let new = ShortPosition::try_from(position)?;
            shorts.push(new);
        }

        let total = shorts
            .iter()
            .map(|e| e.weight)
            .reduce(|acc, e| acc + e)
            .unwrap_or_default();

        let alive_positions = AliveShortPositions {
            total,
            positions: shorts,
            date: Utc::now(),
        };

        Ok(alive_positions)
    }
}

/// Copy of [finance_ibex::IbexCompany] wrapping all attributes with an `Option`.
///
/// # Description
///
/// QuestDB requires all the fields as optional when attempting a casting between a `Row` and
/// a Rust type.
#[derive(Debug, sqlx::FromRow)]
struct IbexCompanyBd {
    pub full_name: Option<String>,
    pub name: Option<String>,
    pub ticker: Option<String>,
    pub isin: Option<String>,
    pub extra_id: Option<String>,
}

/// Allow to cast companies from the DB into `Struct`'s from the library.
impl TryFrom<&IbexCompanyBd> for IbexCompany {
    type Error = DbError;

    fn try_from(value: &IbexCompanyBd) -> Result<Self, Self::Error> {
        let sname = match value.name.as_deref() {
            Some(name) => name,
            None => {
                return Err(DbError::MissingStockInfo(format!(
                    "Missing name: {:?}",
                    value
                )))
            }
        };
        let fname = value.full_name.as_deref();

        let ticker = match value.ticker.as_deref() {
            Some(ticker) => ticker,
            None => {
                return Err(DbError::MissingStockInfo(format!(
                    "Missing ticker: {:?}",
                    value
                )))
            }
        };

        let isin = match value.isin.as_deref() {
            Some(isin) => isin,
            None => {
                return Err(DbError::MissingStockInfo(format!(
                    "Missing ISIN: {:?}",
                    value
                )))
            }
        };

        let nif = value.extra_id.as_deref();

        Ok(IbexCompany::new(fname, sname, ticker, isin, nif))
    }
}

// Mirror data object of [data_harvest::domain::ShortPosition] to interact with the DB.
#[derive(Debug, sqlx::FromRow)]
pub struct ShortPositionBd {
    pub id: Option<Uuid>,
    pub owner: Option<String>,
    pub ticker: Option<String>,
    pub weight: Option<f32>,
    pub open_date: Option<NaiveDateTime>,
}

impl TryFrom<ShortPositionBd> for ShortPosition {
    type Error = DbError;

    fn try_from(value: ShortPositionBd) -> Result<Self, Self::Error> {
        let owner = match value.owner {
            Some(o) => o,
            None => return Err(DbError::MissingStockInfo("Missing owner".to_owned())),
        };

        let weight = match value.weight {
            Some(w) => w,
            None => return Err(DbError::MissingStockInfo("Missing weight".to_owned())),
        };

        let open_date = match value.open_date {
            Some(o) => {
                // Time is kept in UTC within the DB. Left the code in case this changes in the future.
                let tz_offset = FixedOffset::west_opt(0).unwrap();
                let dt_with_tz: DateTime<FixedOffset> = tz_offset.from_local_datetime(&o).unwrap();
                Utc.from_utc_datetime(&dt_with_tz.naive_utc())
            }
            None => return Err(DbError::MissingStockInfo("Missing open date".to_owned())),
        };

        let ticker = match value.ticker {
            Some(t) => t,
            None => return Err(DbError::MissingStockInfo("Missing ticker".to_owned())),
        };

        Ok(ShortPosition {
            owner,
            weight,
            open_date,
            ticker,
        })
    }
}

impl TryFrom<&ShortPositionBd> for ShortPosition {
    type Error = DbError;

    fn try_from(value: &ShortPositionBd) -> Result<Self, Self::Error> {
        let owner = match &value.owner {
            Some(o) => o.to_owned(),
            None => return Err(DbError::MissingStockInfo("Missing owner".to_owned())),
        };

        let weight = match value.weight {
            Some(w) => w,
            None => return Err(DbError::MissingStockInfo("Missing weight".to_owned())),
        };

        let open_date = match value.open_date {
            Some(o) => {
                let tz_offset = FixedOffset::west_opt(0).unwrap();
                let dt_with_tz: DateTime<FixedOffset> = tz_offset.from_local_datetime(&o).unwrap();
                Utc.from_utc_datetime(&dt_with_tz.naive_utc())
            }
            None => return Err(DbError::MissingStockInfo("Missing open date".to_owned())),
        };

        let ticker = match &value.ticker {
            Some(t) => t.to_owned(),
            None => return Err(DbError::MissingStockInfo("Missing ticker".to_owned())),
        };

        Ok(ShortPosition {
            owner,
            weight,
            open_date,
            ticker,
        })
    }
}
