use std::sync::Arc;

use axum::extract::State;
use axum::response::Json;

use crate::db::state as db_state;
use crate::db::usage as db_usage;
use crate::models::types::*;
use crate::api::data::AppState;

/// GET /api/historical
pub async fn api_historical(
    State(state): axum::extract::State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let today_start = crate::api::data::get_today_start();
    let usage_db_path = state.hermes_home.join("usage.db");
    let state_db_path = state.hermes_home.join("state.db");

    let result = (|| -> Result<HistoricalData, String> {
        let conn_usage = db_usage::open(&usage_db_path).map_err(|e| format!("usage.db: {}", e))?;
        let conn_state = db_state::open(&state_db_path).map_err(|e| format!("state.db: {}", e))?;
        crate::api::data::get_historical_data(&conn_usage, &conn_state, today_start)
    })();

    match result {
        Ok(data) => Json(serde_json::to_value(data).unwrap_or_default()),
        Err(e) => Json(serde_json::json!({"error": e})),
    }
}
