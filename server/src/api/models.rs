use std::sync::Arc;

use axum::extract::State;
use axum::response::Json;

use crate::db::usage as db_usage;
use crate::models::types::ModelSummary;
use crate::api::data::AppState;

/// GET /api/models
pub async fn api_models(
    State(state): axum::extract::State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let usage_db_path = state.hermes_home.join("usage.db");
    let state_db_path = state.hermes_home.join("state.db");

    let result = (|| -> Result<(Vec<String>, std::collections::BTreeMap<String, ModelSummary>), String> {
        let conn_usage = db_usage::open(&usage_db_path).map_err(|e| format!("usage.db: {}", e))?;
        let conn_state = crate::db::state::open(&state_db_path).map_err(|e| format!("state.db: {}", e))?;
        let models = db_usage::get_models(&conn_usage).map_err(|e| format!("models: {}", e))?;
        let summaries = db_usage::get_model_summaries(&conn_usage, &conn_state)
            .map_err(|e| format!("summaries: {}", e))?;
        Ok((models, summaries))
    })();

    match result {
        Ok((models, summaries)) => Json(serde_json::json!({
            "models": models,
            "summaries": summaries,
        })),
        Err(e) => Json(serde_json::json!({"error": e})),
    }
}
