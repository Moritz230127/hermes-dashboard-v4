use std::sync::Arc;

use axum::extract::State;
use axum::response::Json;

use crate::db::{state as db_state, usage as db_usage};
use crate::models::types::*;
use crate::api::data::AppState;

/// GET /api/alerts
pub async fn api_alerts(
    State(state): axum::extract::State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let today_start = crate::api::data::get_today_start();
    let usage_db_path = state.hermes_home.join("usage.db");
    let state_db_path = state.hermes_home.join("state.db");

    let result = (|| -> Result<Vec<Alert>, String> {
        let conn_usage = db_usage::open(&usage_db_path).map_err(|e| format!("usage.db: {}", e))?;
        let conn_state = db_state::open(&state_db_path).map_err(|e| format!("state.db: {}", e))?;
        let alerts = crate::api::data::check_alerts(&conn_usage, &conn_state, today_start, &state.thresholds)?;
        Ok(alerts)
    })();

    match result {
        Ok(alerts) => Json(serde_json::json!({"alerts": alerts})),
        Err(e) => Json(serde_json::json!({"error": e})),
    }
}
