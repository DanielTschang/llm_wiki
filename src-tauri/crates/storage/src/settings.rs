//! Organisation-wide settings and the record of LLM calls.

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::sql::backend;
use crate::StoreError;

/// The key under which the model catalogue is kept.
pub const LLM_MODELS_KEY: &str = "llm.models";

/// `server_settings`: non-secret configuration that every user's session
/// reads and only a Platform Admin changes. API keys never live here.
#[derive(Clone)]
pub struct ServerSettings {
    pool: PgPool,
}

impl ServerSettings {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get(&self, key: &str) -> Result<Option<Value>, StoreError> {
        sqlx::query_scalar("SELECT value FROM server_settings WHERE key = $1")
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)
    }

    pub async fn set(&self, key: &str, value: &Value, by: Option<Uuid>) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO server_settings (key, value, updated_by) VALUES ($1, $2, $3)
             ON CONFLICT (key) DO UPDATE
             SET value = EXCLUDED.value, updated_by = EXCLUDED.updated_by, updated_at = now()",
        )
        .bind(key)
        .bind(value)
        .bind(by)
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(())
    }
}

/// What the gateway records about one call.
#[derive(Debug, Clone, Default)]
pub struct UsageRecord {
    pub user_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub model_id: String,
    pub status: u16,
    pub duration_ms: u64,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

/// Totals for one user and model over a period.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageTotal {
    pub user_id: Option<Uuid>,
    pub display_name: Option<String>,
    pub model_id: String,
    pub calls: i64,
    pub failed: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

#[derive(Clone)]
pub struct LlmUsage {
    pool: PgPool,
}

impl LlmUsage {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn record(&self, usage: &UsageRecord) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO llm_usage (user_id, project_id, model_id, status, duration_ms,
                                    bytes_in, bytes_out, input_tokens, output_tokens)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(usage.user_id)
        .bind(usage.project_id)
        .bind(&usage.model_id)
        .bind(i32::from(usage.status))
        .bind(usage.duration_ms.min(i32::MAX as u64) as i32)
        .bind(usage.bytes_in as i64)
        .bind(usage.bytes_out as i64)
        .bind(usage.input_tokens.map(|n| n as i64))
        .bind(usage.output_tokens.map(|n| n as i64))
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    /// Calls and tokens per user and model since `since`, busiest first.
    pub async fn totals_since(&self, since: DateTime<Utc>) -> Result<Vec<UsageTotal>, StoreError> {
        let rows = sqlx::query(
            "SELECT l.user_id, u.display_name, l.model_id,
                    count(*) AS calls,
                    count(*) FILTER (WHERE l.status >= 400) AS failed,
                    coalesce(sum(l.input_tokens), 0)::bigint AS input_tokens,
                    coalesce(sum(l.output_tokens), 0)::bigint AS output_tokens
             FROM llm_usage l LEFT JOIN users u ON u.id = l.user_id
             WHERE l.created_at >= $1
             GROUP BY l.user_id, u.display_name, l.model_id
             ORDER BY calls DESC, l.model_id",
        )
        .bind(since)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        rows.iter()
            .map(|row| {
                Ok(UsageTotal {
                    user_id: row.try_get("user_id").map_err(backend)?,
                    display_name: row.try_get("display_name").map_err(backend)?,
                    model_id: row.try_get("model_id").map_err(backend)?,
                    calls: row.try_get("calls").map_err(backend)?,
                    failed: row.try_get("failed").map_err(backend)?,
                    input_tokens: row.try_get("input_tokens").map_err(backend)?,
                    output_tokens: row.try_get("output_tokens").map_err(backend)?,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use sqlx::postgres::PgPoolOptions;

    use super::*;

    async fn pool() -> Option<PgPool> {
        let url = std::env::var("LLM_WIKI_TEST_DATABASE_URL").ok()?;
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect(&url)
            .await
            .unwrap();
        crate::migrate(&pool).await.unwrap();
        Some(pool)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn settings_are_replaced_in_place_and_remember_who_changed_them() {
        let Some(pool) = pool().await else { return };
        let settings = ServerSettings::new(pool);
        let key = format!("test.{}", Uuid::new_v4());
        assert_eq!(settings.get(&key).await.unwrap(), None);
        settings
            .set(&key, &serde_json::json!([1]), None)
            .await
            .unwrap();
        settings
            .set(&key, &serde_json::json!({ "a": 2 }), None)
            .await
            .unwrap();
        assert_eq!(
            settings.get(&key).await.unwrap(),
            Some(serde_json::json!({ "a": 2 }))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn usage_is_totalled_per_user_and_model() {
        let Some(pool) = pool().await else { return };
        let usage = LlmUsage::new(pool);
        let model = format!("model-{}", Uuid::new_v4());
        let call = |status, input, output| UsageRecord {
            model_id: model.clone(),
            status,
            duration_ms: 120,
            bytes_in: 10,
            bytes_out: 20,
            input_tokens: input,
            output_tokens: output,
            ..UsageRecord::default()
        };
        let before = Utc::now() - chrono::Duration::seconds(1);
        usage.record(&call(200, Some(100), Some(40))).await.unwrap();
        usage.record(&call(200, Some(50), None)).await.unwrap();
        usage.record(&call(429, None, None)).await.unwrap();
        let totals = usage.totals_since(before).await.unwrap();
        let mine: Vec<_> = totals.into_iter().filter(|t| t.model_id == model).collect();
        assert_eq!(mine.len(), 1);
        assert_eq!(
            (
                mine[0].calls,
                mine[0].failed,
                mine[0].input_tokens,
                mine[0].output_tokens
            ),
            (3, 1, 150, 40)
        );
    }
}
