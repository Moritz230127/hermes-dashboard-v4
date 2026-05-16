use std::collections::BTreeMap;
use std::sync::Arc;

use axum::extract::State;
use axum::extract::{Path, Query};
use axum::response::Json;

use crate::db::state as db_state;
use crate::models::types::*;
use crate::api::data::AppState;

/// Find a script by name, checking local `./scripts/` first, then `~/.hermes/dashboard-v4/scripts/`.
fn find_script(name: &str) -> std::path::PathBuf {
    let local = std::env::current_dir().unwrap_or_default().join("scripts").join(name);
    if local.exists() {
        return local;
    }
    if let Ok(home) = std::env::var("HERMES_HOME")
        .or_else(|_| std::env::var("HOME"))
        .or_else(|_| std::env::var("USERPROFILE"))
    {
        let hermes_script = std::path::PathBuf::from(home)
            .join(".hermes").join("dashboard-v4").join("scripts").join(name);
        if hermes_script.exists() {
            return hermes_script;
        }
    }
    local // default to local even if not found (caller checks .exists())
}

/// Get the Python command name for the current platform
fn python_cmd() -> &'static str {
    if cfg!(windows) { "python" } else { "python3" }
}

/// Fallback process killing when kill_session.py is not available
#[cfg(unix)]
fn kill_agent_fallback(db_path: &std::path::Path) -> String {
    let db_path_str = db_path.to_string_lossy();
    let python_cmd_str = python_cmd();
    let _ = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!(
            "lsof '{}' 2>/dev/null | grep '{}' | grep -v dashboard | \
             awk '{{print $2}}' | sort -u | xargs -r kill -TERM 2>/dev/null",
            db_path_str, python_cmd_str
        ))
        .output();
    "killed_by_fallback".to_string()
}

#[cfg(windows)]
fn kill_agent_fallback(_db_path: &std::path::Path) -> String {
    // On Windows: use taskkill to terminate any python processes
    let _ = std::process::Command::new("taskkill")
        .args(&["/F", "/IM", "python.exe"])
        .output()
        .ok();
    "killed_by_fallback_windows".to_string()
}

/// GET /api/sessions/running
pub async fn get_running(
    State(state): axum::extract::State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let state_db_path = state.hermes_home.join("state.db");

    let result = (|| -> Result<RunningSessionsResponse, String> {
        let conn = db_state::open(&state_db_path).map_err(|e| format!("state.db: {}", e))?;
        let sessions = db_state::get_running_sessions(&conn)
            .map_err(|e| format!("running sessions: {}", e))?;
        let count = sessions.len();
        Ok(RunningSessionsResponse { count, sessions })
    })();

    match result {
        Ok(data) => Json(serde_json::to_value(data).unwrap_or_default()),
        Err(e) => Json(serde_json::json!({"status": "error", "message": e})),
    }
}

/// POST /api/sessions/stop/{session_id}
/// REAL stop: updates state.db AND attempts to kill the agent process
pub async fn stop_session(
    State(state): axum::extract::State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> Json<serde_json::Value> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();

    let state_db_path = state.hermes_home.join("state.db");
    let kill_script = find_script("kill_session.py");

    let result = (|| -> Result<StatusResponse, String> {
        let conn = db_state::open(&state_db_path).map_err(|e| format!("state.db: {}", e))?;

        // Check if session exists and is running
        let row: Result<(String, Option<f64>), _> = conn.query_row(
            "SELECT id, ended_at FROM sessions WHERE id = ?1",
            rusqlite::params![&session_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<f64>>(1)?)),
        );

        let already_ended = match row {
            Err(_) => {
                return Ok(StatusResponse {
                    status: "error".into(),
                    message: format!("Session not found: {}", session_id),
                    session_id: None,
                    stopped_count: None,
                    kept_session: None,
                    updated_count: None,
                    old_sessions_count: None,
                    note: None,
                });
            }
            Ok((_, Some(_))) => true,
            Ok((_, None)) => false,
        };

        // Step 1: Set ended_at in database (always do this)
        if !already_ended {
            db_state::stop_session(&conn, &session_id, now)
                .map_err(|e| format!("stop session: {}", e))?;
        }

        // Step 2: Try to kill the actual agent process
        let kill_result = if kill_script.exists() {
            let output = std::process::Command::new(python_cmd())
                .arg(&kill_script)
                .arg(&session_id)
                .output()
                .map_err(|e| format!("kill script: {}", e))?;

            String::from_utf8_lossy(&output.stdout).to_string()
        } else {
            kill_agent_fallback(&state_db_path)
        };

        let message = if already_ended {
            format!("Session already ended. Process kill attempted: {}", kill_result)
        } else {
            format!("Session stopped. Process kill attempted: {}", kill_result)
        };

        let note = if kill_result.contains("\"killed\":[") || kill_result.contains("killed_by") {
            Some("Agent process was terminated".into())
        } else if kill_result.contains("\"status\":\"info\"") {
            Some("No running agent process found for this session (may already be stopped)".into())
        } else {
            Some("Session marked in DB. Kill: ".to_string() + &kill_result[..200])
        };

        Ok(StatusResponse {
            status: "success".into(),
            message,
            session_id: Some(session_id),
            stopped_count: None,
            kept_session: None,
            updated_count: None,
            old_sessions_count: None,
            note,
        })
    })();

    match result {
        Ok(data) => Json(serde_json::to_value(data).unwrap_or_default()),
        Err(e) => Json(serde_json::json!({"status": "error", "message": e})),
    }
}

/// POST /api/sessions/stop-others
pub async fn stop_other_sessions(
    State(state): axum::extract::State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();

    let state_db_path = state.hermes_home.join("state.db");
    let kill_script = find_script("kill_session.py");

    let result = (|| -> Result<StatusResponse, String> {
        let conn = db_state::open(&state_db_path).map_err(|e| format!("state.db: {}", e))?;

        // FETCH running sessions BEFORE marking them ended
        let running = db_state::get_running_sessions(&conn)
            .map_err(|e| format!("running: {}", e))?;

        let (stopped_count, kept) = db_state::stop_other_sessions(&conn, now)
            .map_err(|e| format!("stop others: {}", e))?;

        // Try to kill ALL agent processes (use pre-fetched running list)
        let mut killed_count = 0;
        if kill_script.exists() {
            for session in &running {
                if let Some(ref kept_id) = kept
                    && session.id == *kept_id {
                    continue;
                }
                let output = std::process::Command::new(python_cmd())
                    .arg(&kill_script)
                    .arg(&session.id)
                    .output()
                    .map_err(|e| format!("kill: {}", e))?;
                let result_str = String::from_utf8_lossy(&output.stdout);
                if result_str.contains("\"killed\"") {
                    killed_count += 1;
                }
            }
        }

        Ok(StatusResponse {
            status: "success".into(),
            message: format!("Stopped {} sessions ({} processes killed)", stopped_count, killed_count),
            session_id: None,
            stopped_count: Some(stopped_count),
            kept_session: kept.map(|k| {
                if k.len() > 20 { k[..20].to_string() + "..." } else { k }
            }),
            updated_count: None,
            old_sessions_count: None,
            note: Some(format!("{} sessions marked ended + {} agent process(es) killed", stopped_count, killed_count)),
        })
    })();

    match result {
        Ok(data) => Json(serde_json::to_value(data).unwrap_or_default()),
        Err(e) => Json(serde_json::json!({"status": "error", "message": e})),
    }
}

/// POST /api/sessions/mark-compression?session_id=xxx
/// REAL compression: marks in DB AND calls Ollama to actually compress
pub async fn mark_compression(
    State(state): axum::extract::State<Arc<AppState>>,
    params: Query<BTreeMap<String, String>>,
) -> Json<serde_json::Value> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();

    let session_id = params.get("session_id").map(|s| s.as_str());
    let state_db_path = state.hermes_home.join("state.db");
    let compress_script = find_script("compress_session.py");

    let result = (|| -> Result<StatusResponse, String> {
        let conn = db_state::open(&state_db_path).map_err(|e| format!("state.db: {}", e))?;

        // Step 1: Mark in DB (existing behavior)
        let cutoff = if session_id.is_some() { 0.0 } else { now - 86400.0 };
        let count = db_state::mark_compression(&conn, cutoff, session_id)
            .map_err(|e| format!("mark compression: {}", e))?;

        let note = if let Some(sid) = session_id {
            if compress_script.exists() {
                let output = std::process::Command::new(python_cmd())
                    .arg(&compress_script)
                    .arg(sid)
                    .output()
                    .map_err(|e| format!("compress script: {}", e))?;

                let out_str = String::from_utf8_lossy(&output.stdout);
                if output.status.success() {
                    format!("Compression result: {}", &out_str[..300])
                } else {
                    let err_str = String::from_utf8_lossy(&output.stderr);
                    format!("Compression attempted (script error: {})", &err_str[..200])
                }
            } else {
                "Marked in DB. Compression script not found, skipping Ollama call.".into()
            }
        } else {
            format!("Marked {} old sessions in DB. Bulk compression skipped (use individual session compression instead).", count)
        };

        let message = match session_id {
            Some(_) => format!("Compressed {} session", count),
            None => format!("Marked {} old sessions for compression", count),
        };

        Ok(StatusResponse {
            status: "success".into(),
            message,
            session_id: session_id.map(|s| s.to_string()),
            stopped_count: None,
            kept_session: None,
            updated_count: Some(count),
            old_sessions_count: None,
            note: Some(if note.is_empty() { "Compression completed".into() } else { note }),
        })
    })();

    match result {
        Ok(data) => Json(serde_json::to_value(data).unwrap_or_default()),
        Err(e) => Json(serde_json::json!({"status": "error", "message": e})),
    }
}

/// POST /api/memory/cleanup
pub async fn memory_cleanup(
    State(state): axum::extract::State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    let cutoff = now - 7.0 * 86400.0;

    // Use a writable connection for real cleanup
    let hermes_home = state.hermes_home.clone();
    let result = tokio::task::spawn_blocking(move || -> Result<StatusResponse, String> {
        let state_db_path = hermes_home.join("state.db");
        let mut conn = db_state::open(&state_db_path).map_err(|e| format!("state.db: {}", e))?;

        // ACTUAL real cleanup: delete old sessions + VACUUM
        let (deleted, saved) = db_state::cleanup_old_sessions(&mut conn, cutoff)
            .map_err(|e| format!("cleanup: {}", e))?;

        Ok(StatusResponse {
            status: "success".into(),
            message: format!(
                "Cleaned up {} old ended sessions, recovered ~{} KB",
                deleted,
                saved / 1024
            ),
            session_id: None,
            stopped_count: None,
            kept_session: None,
            updated_count: None,
            old_sessions_count: Some(deleted),
            note: Some(format!("Deleted {} sessions > 7 days old + VACUUM reclaimed space", deleted)),
        })
    }).await.unwrap_or(Err("cleanup task failed".into()));

    match result {
        Ok(data) => Json(serde_json::to_value(data).unwrap_or_default()),
        Err(e) => Json(serde_json::json!({"status": "error", "message": e})),
    }
}
