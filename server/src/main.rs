mod api;
mod db;
mod models;

use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use api::data::AppState;
use models::types::AlertThresholds;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let hermes_home = std::env::var("HERMES_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .unwrap_or_else(|_| "/root".to_string());
            PathBuf::from(home).join(".hermes")
        });

    let port: u16 = std::env::var("HERMES_DASHBOARD_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8654);

    let host: String = std::env::var("HERMES_DASHBOARD_HOST")
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    let dist_path = hermes_home
        .join("dashboard-v4")
        .join("dist");
    let dist_path = if dist_path.exists() {
        dist_path
    } else {
        // fallback: 开发模式 / 从仓库根目录运行
        std::env::current_dir()
            .unwrap_or_default()
            .join("dist")
    };

    println!("Hermes API Dashboard V4 (Rust)");
    println!("  http://{}:{}", host, port);
    println!("  API: http://{}:{}/api/data", host, port);
    println!("  Databases: {}", hermes_home.display());
    println!("  V4 Features: Pool Rotator, SSE, Cleanup");
    println!("  Press Ctrl+C to stop");

    let state = Arc::new(AppState {
        hermes_home,
        thresholds: AlertThresholds::default(),
    });

    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    let app = Router::new()
        .route("/api/data", get(api::data::api_data))
        .route("/api/alerts", get(api::alerts::api_alerts))
        .route("/api/health", get(api::health::api_health))
        .route("/api/models", get(api::models::api_models))
        .route("/api/historical", get(api::historical::api_historical))
        .route("/api/sessions/running", get(api::sessions::get_running))
        .route("/api/sessions/stop/{session_id}", post(api::sessions::stop_session))
        .route("/api/sessions/stop-others", post(api::sessions::stop_other_sessions))
        .route("/api/sessions/mark-compression", post(api::sessions::mark_compression))
        .route("/api/memory/cleanup", post(api::sessions::memory_cleanup))
        .layer(cors)
        .fallback_service(ServeDir::new(&dist_path).append_index_html_on_directories(true))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, port))
        .await
        .expect("Failed to bind address");

    axum::serve(listener, app)
        .await
        .expect("Server error");
}
