use std::collections::BTreeMap;
use std::sync::Arc;

use axum::extract::Query;
use axum::response::Json;

use crate::db::{state as db_state, usage as db_usage};
use crate::models::types::*;

pub struct AppState {
    pub hermes_home: std::path::PathBuf,
    pub thresholds: AlertThresholds,
}

/// GET /api/data?model=__all__
pub async fn api_data(
    state: axum::extract::State<Arc<AppState>>,
    params: Query<BTreeMap<String, String>>,
) -> Json<serde_json::Value> {
    let model_filter = params.get("model").map(|s| s.as_str());
    let now = chrono::Local::now().timestamp() as f64;

    let today_start = get_today_start();

    let usage_db_path = state.hermes_home.join("usage.db");
    let state_db_path = state.hermes_home.join("state.db");

    let result = (|| -> Result<ApiDataResponse, String> {
        let conn_usage = db_usage::open(&usage_db_path).map_err(|e| format!("usage.db: {}", e))?;
        let conn_state = db_state::open(&state_db_path).map_err(|e| format!("state.db: {}", e))?;

        let today = query_period(&conn_usage, &conn_state, today_start, None)?;
        let historical = get_historical_data(&conn_usage, &conn_state, today_start)?;
        let hourly = db_usage::query_hourly_today(&conn_usage, today_start, model_filter)
            .map_err(|e| format!("hourly: {}", e))?;
        let buckets_10m = db_usage::query_10min_buckets_today(&conn_usage, today_start, now, model_filter)
            .map_err(|e| format!("10min: {}", e))?;

        let sessions = db_state::get_sessions(&conn_state, today_start - 86400.0, model_filter, 50)
            .map_err(|e| format!("sessions: {}", e))?;

        let models = db_usage::get_models(&conn_usage).map_err(|e| format!("models: {}", e))?;
        let model_summaries = db_usage::get_model_summaries(&conn_usage, &conn_state)
            .map_err(|e| format!("model summaries: {}", e))?;

        // Pool Rotator
        let pool_path = state.hermes_home.join("pool_rotator/state.json");
        let pool_rotator = crate::db::pool_rotator::read_pool_state(&pool_path);

        let alerts = check_alerts(&conn_usage, &conn_state, today_start, &state.thresholds)?;

        let rolling_5h = query_period(&conn_usage, &conn_state, now - 18000.0, None)?;
        let rolling_24h = query_period(&conn_usage, &conn_state, now - 86400.0, None)?;

        let hourly_rolling_24h = db_usage::query_hourly_rolling_24h(&conn_usage, now - 86400.0, now, model_filter)
            .map_err(|e| format!("hourly rolling: {}", e))?;

        let today_start_dt = chrono::DateTime::from_timestamp(today_start as i64, 0)
            .map(|d| d.with_timezone(&chrono::Local))
            .unwrap_or_default();

        Ok(ApiDataResponse {
            models,
            today,
            historical,
            hourly,
            buckets_10m,
            sessions,
            model_summaries,
            pool_rotator,
            alerts,
            now,
            context_window: 128000,
            rolling_5h,
            rolling_24h,
            hourly_rolling_24h,
            config: DashboardConfig {
                today_start: today_start_dt.format("%Y-%m-%d %H:%M:%S").to_string(),
                timezone: "CST".to_string(),
                thresholds: AlertThresholds::default(),
            },
        })
    })();

    match result {
        Ok(data) => Json(serde_json::to_value(data).unwrap_or_default()),
        Err(e) => {
            tracing::error!("API error: {}", e);
            Json(serde_json::json!({
                "error": e,
                "now": now,
            }))
        }
    }
}

pub fn get_today_start() -> f64 {
    let now = chrono::Local::now();
    let today = now.date_naive();
    today.and_hms_opt(0, 0, 0)
        .map(|dt| dt.and_local_timezone(chrono::Local).unwrap())
        .map(|dt| dt.timestamp() as f64)
        .unwrap_or(0.0)
}

pub fn get_yesterday_start() -> f64 {
    get_day_start(1)
}

pub fn get_day_start(days_ago: i64) -> f64 {
    let now = chrono::Local::now();
    let day = now.date_naive() - chrono::Days::new(days_ago as u64);
    day.and_hms_opt(0, 0, 0)
        .map(|dt| dt.and_local_timezone(chrono::Local).unwrap())
        .map(|dt| dt.timestamp() as f64)
        .unwrap_or(0.0)
}

pub fn query_period(
    conn_usage: &rusqlite::Connection,
    conn_state: &rusqlite::Connection,
    cutoff_start: f64,
    cutoff_end: Option<f64>,
) -> Result<PeriodStats, String> {
    let core = db_usage::query_period_core(conn_usage, cutoff_start, cutoff_end)
        .map_err(|e| format!("core: {}", e))?;
    let aux = db_state::query_period_aux(conn_state, cutoff_start, cutoff_end)
        .map_err(|e| format!("aux: {}", e))?;
    Ok(PeriodStats::compute(&core, &aux))
}

pub fn get_historical_data(
    conn_usage: &rusqlite::Connection,
    conn_state: &rusqlite::Connection,
    today_start: f64,
) -> Result<HistoricalData, String> {
    let yesterday_start = get_yesterday_start();
    let day3_start = get_day_start(2);
    let day4_start = get_day_start(3);
    let day5_start = get_day_start(4);
    let day6_start = get_day_start(5);
    let day7_start = get_day_start(6);

    let today = query_period(conn_usage, conn_state, today_start, None)?;
    let yesterday = query_period(conn_usage, conn_state, yesterday_start, Some(today_start))?;
    let day3 = query_period(conn_usage, conn_state, day3_start, Some(yesterday_start))?;
    let day4 = query_period(conn_usage, conn_state, day4_start, Some(day3_start))?;
    let day5 = query_period(conn_usage, conn_state, day5_start, Some(day4_start))?;
    let day6 = query_period(conn_usage, conn_state, day6_start, Some(day5_start))?;
    let day7 = query_period(conn_usage, conn_state, day7_start, Some(day6_start))?;

    let last_7 = [&yesterday, &day3, &day4, &day5, &day6, &day7];

    let avg = |field: fn(&PeriodStats) -> f64| -> f64 {
        let values: Vec<f64> = last_7.iter().map(|d| field(d)).collect();
        let sum: f64 = values.iter().sum();
        if !values.is_empty() { sum / values.len() as f64 } else { 0.0 }
    };

    let avg_7d = PeriodStats {
        api_calls: avg(|s| s.api_calls as f64) as i64,
        sessions: avg(|s| s.sessions as f64) as i64,
        total_tokens: avg(|s| s.total_tokens as f64) as i64,
        input_tokens: avg(|s| s.input_tokens as f64) as i64,
        output_tokens: avg(|s| s.output_tokens as f64) as i64,
        cache_read_tokens: avg(|s| s.cache_read_tokens as f64) as i64,
        messages: avg(|s| s.messages as f64) as i64,
        tool_calls: avg(|s| s.tool_calls as f64) as i64,
        premature: avg(|s| s.premature as f64) as i64,
        avg_tps: avg(|s| s.avg_tps),
        ..Default::default()
    };

    let fmt = |days_ago: i64| -> String {
        let dt = chrono::Local::now() - chrono::Days::new(days_ago as u64);
        dt.format("%m-%d").to_string()
    };

    let mut daily_breakdown = BTreeMap::new();
    daily_breakdown.insert("today".into(), DailyItem { date: fmt(0), data: today.clone() });
    daily_breakdown.insert("yesterday".into(), DailyItem { date: fmt(1), data: yesterday.clone() });
    daily_breakdown.insert("2d_ago".into(), DailyItem { date: fmt(2), data: day3.clone() });
    daily_breakdown.insert("3d_ago".into(), DailyItem { date: fmt(3), data: day4.clone() });
    daily_breakdown.insert("4d_ago".into(), DailyItem { date: fmt(4), data: day5.clone() });
    daily_breakdown.insert("5d_ago".into(), DailyItem { date: fmt(5), data: day6.clone() });
    daily_breakdown.insert("6d_ago".into(), DailyItem { date: fmt(6), data: day7.clone() });

    // Compute daily_30: last 30 days of daily stats
    let mut daily_30 = Vec::new();
    for day in 1..=30 {
        let day_end = get_day_start(day - 1);
        let day_start = get_day_start(day);
        let stats = query_period(conn_usage, conn_state, day_start, Some(day_end))?;
        daily_30.push(DailyItem {
            date: fmt(day),
            data: stats,
        });
    }
    // Reverse so oldest first
    daily_30.reverse();

    Ok(HistoricalData {
        today,
        yesterday,
        day_before_yesterday: day3,
        avg_7d,
        daily_breakdown,
        daily_30,
    })
}

pub fn check_alerts(
    conn_usage: &rusqlite::Connection,
    conn_state: &rusqlite::Connection,
    today_start: f64,
    thresholds: &AlertThresholds,
) -> Result<Vec<Alert>, String> {
    let today = query_period(conn_usage, conn_state, today_start, None)?;
    let mut alerts = Vec::new();

    // Alert 1: Low TPS
    if today.avg_tps > 0.0 && today.avg_tps < thresholds.min_tps {
        alerts.push(Alert {
            level: "warning".into(),
            category: "performance".into(),
            message: "Average TPS is below threshold".into(),
            value: Some((today.avg_tps * 100.0).round() / 100.0),
            threshold: Some(thresholds.min_tps),
        });
    }

    // Alert 2: High premature termination
    if today.sessions > 0 {
        let premature_rate = today.premature as f64 / today.sessions as f64 * 100.0;
        if premature_rate > thresholds.max_premature_rate {
            alerts.push(Alert {
                level: "error".into(),
                category: "stability".into(),
                message: "High premature termination rate".into(),
                value: Some((premature_rate * 10.0).round() / 10.0),
                threshold: Some(thresholds.max_premature_rate),
            });
        }
    }

    // Alert 3: Low cache hit rate
    let total = today.input_tokens + today.cache_read_tokens;
    if total > 0 {
        let cache_rate = today.cache_read_tokens as f64 / total as f64 * 100.0;
        if cache_rate > 0.0 && cache_rate < thresholds.min_cache_hit_rate {
            alerts.push(Alert {
                level: "info".into(),
                category: "efficiency".into(),
                message: "Cache hit rate could be improved".into(),
                value: Some((cache_rate * 10.0).round() / 10.0),
                threshold: Some(thresholds.min_cache_hit_rate),
            });
        }
    }

    // Alert 4: API spike vs 7-day average
    let historical = get_historical_data(conn_usage, conn_state, today_start)?;
    let avg_api = historical.avg_7d.api_calls;
    if avg_api > 10 {
        let ratio = today.api_calls as f64 / avg_api as f64;
        if ratio > thresholds.max_api_spike_ratio {
            alerts.push(Alert {
                level: "warning".into(),
                category: "activity".into(),
                message: "API calls significantly above 7-day average".into(),
                value: Some((ratio * 100.0).round() / 100.0),
                threshold: Some(thresholds.max_api_spike_ratio),
            });
        } else if ratio < 0.3 {
            alerts.push(Alert {
                level: "info".into(),
                category: "activity".into(),
                message: "API calls significantly below 7-day average".into(),
                value: Some((ratio * 100.0).round() / 100.0),
                threshold: Some(0.3),
            });
        }
    }

    Ok(alerts)
}
