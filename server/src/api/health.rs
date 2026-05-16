use std::sync::Arc;

use axum::extract::State;
use axum::response::Json;

use crate::models::types::HealthResponse;
use crate::api::data::AppState;

/// GET /api/health
pub async fn api_health(
    State(state): axum::extract::State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();

    let usage_db_path = state.hermes_home.join("usage.db");
    let state_db_path = state.hermes_home.join("state.db");

    let healthy = usage_db_path.exists() && state_db_path.exists();

    if healthy {
        Json(serde_json::to_value(HealthResponse {
            status: "healthy".into(),
            timestamp: now,
            version: "4.0.0".into(),
        }).unwrap_or_default())
    } else {
        Json(serde_json::json!({
            "status": "unhealthy",
            "timestamp": now,
            "version": "4.0.0",
            "error": "Database not found"
        }))
    }
}
