use crate::pg::Database;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use thiserror::Error;

#[derive(Debug, Serialize, FromRow)]
pub struct LiquidityPoint {
    pub bucket_ts: DateTime<Utc>,
    pub avg_bps: f64,
}

#[derive(Debug, Error)]
pub enum LiquidityError {
    #[error("Failed to query liquidity data: {0}")]
    QueryFailed(#[from] sqlx::Error),
}

#[derive(Clone, Debug)]
pub struct LiquidityService {
    db: Database,
}

impl LiquidityService {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn get_liquidity(
        &self,
        symbol: &str,
        order_value: i64,
        from_ts: DateTime<Utc>,
        to_ts: DateTime<Utc>,
    ) -> Result<Vec<LiquidityPoint>, LiquidityError> {
        let points = sqlx::query_as::<_, LiquidityPoint>(
            "SELECT bucket_ts, avg_bps
             FROM exec_cost_1hour
             WHERE symbol = $1
               AND order_value = $2
               AND bucket_ts >= $3
               AND bucket_ts <= $4
             ORDER BY bucket_ts",
        )
        .bind(symbol)
        .bind(order_value)
        .bind(from_ts)
        .bind(to_ts)
        .fetch_all(self.db.pool())
        .await?;

        Ok(points)
    }
}
