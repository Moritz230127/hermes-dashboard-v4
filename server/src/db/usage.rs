use rusqlite::Connection;
use std::path::Path;

use crate::models::types::CoreStats;

/// Open usage.db connection
pub fn open(db_path: &Path) -> rusqlite::Result<Connection> {
    Connection::open(db_path)
}

/// Query core metrics for a time period.
pub fn query_period_core(
    conn: &Connection,
    cutoff_start: f64,
    cutoff_end: Option<f64>,
) -> rusqlite::Result<CoreStats> {
    let (api_calls, input_tokens, output_tokens, sessions) = match cutoff_end {
        Some(end) => conn.query_row(
            "SELECT
                COALESCE(SUM(api_calls), 0),
                COALESCE(SUM(prompt_tokens), 0),
                COALESCE(SUM(completion_tokens), 0),
                COALESCE(COUNT(DISTINCT session_id), 0)
             FROM api_usage_logs
             WHERE timestamp >= ?1 AND timestamp < ?2",
            rusqlite::params![cutoff_start, end],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )?,
        None => conn.query_row(
            "SELECT
                COALESCE(SUM(api_calls), 0),
                COALESCE(SUM(prompt_tokens), 0),
                COALESCE(SUM(completion_tokens), 0),
                COALESCE(COUNT(DISTINCT session_id), 0)
             FROM api_usage_logs
             WHERE timestamp >= ?1",
            rusqlite::params![cutoff_start],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )?,
    };

    Ok(CoreStats {
        api_calls,
        input_tokens,
        output_tokens,
        sessions,
    })
}

fn query_rows_by_model(
    conn: &Connection,
    today_start: f64,
    model: Option<&str>,
    sql_all: &str,
    sql_filtered: &str,
) -> rusqlite::Result<Vec<(f64, i64, i64)>> {
    let (sql, params_raw): (&str, Vec<String>) = match model {
        Some(m) if m != "__all__" => {
            (sql_filtered, vec![today_start.to_string(), m.to_string()])
        }
        _ => {
            (sql_all, vec![today_start.to_string()])
        }
    };

    let mut stmt = conn.prepare(sql)?;
    let params: Vec<&dyn rusqlite::types::ToSql> =
        params_raw.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
    let rows = stmt.query_map(params.as_slice(), |row| {
        Ok((
            row.get::<_, f64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Query hourly buckets for today (00:00 to now, full 24h).
pub fn query_hourly_today(
    conn: &Connection,
    today_start: f64,
    model: Option<&str>,
) -> rusqlite::Result<std::collections::BTreeMap<String, crate::models::types::Bucket>> {
    use crate::models::types::Bucket;
    #[allow(unused_imports)] use chrono::TimeZone;
    let local_tz = chrono::Local;
    let mut hourly: std::collections::BTreeMap<String, Bucket> = std::collections::BTreeMap::new();

    // Initialize 24 hours in local timezone
    for h in 0..24 {
        let label = format!("{:02}:00", h);
        hourly.insert(label, Bucket { api_calls: 0, tokens: 0 });
    }

    let rows = query_rows_by_model(
        conn,
        today_start,
        model,
        "SELECT timestamp, api_calls, prompt_tokens + completion_tokens
         FROM api_usage_logs WHERE timestamp >= ?1",
        "SELECT timestamp, api_calls, prompt_tokens + completion_tokens
         FROM api_usage_logs WHERE timestamp >= ?1 AND model = ?2",
    )?;

    for (ts, cnt, tok) in rows {
        let dt = chrono::DateTime::from_timestamp(ts as i64, 0)
            .map(|d| d.with_timezone(&local_tz))
            .map(|d| d.format("%H:00").to_string())
            .unwrap_or_else(|| "00:00".to_string());
        if let Some(b) = hourly.get_mut(&dt) {
            b.api_calls += cnt;
            b.tokens += tok;
        }
    }

    Ok(hourly)
}

/// Query 10-minute buckets for today (00:00 to now).
pub fn query_10min_buckets_today(
    conn: &Connection,
    today_start: f64,
    now: f64,
    model: Option<&str>,
) -> rusqlite::Result<std::collections::BTreeMap<String, crate::models::types::Bucket>> {
    use crate::models::types::Bucket;
    #[allow(unused_imports)]
    use chrono::TimeZone;
    let local_tz = chrono::Local;

    let now_aligned = (now as i64 / 600) * 600;
    let today_aligned = (today_start as i64 / 600) * 600;

    let mut buckets: std::collections::BTreeMap<String, Bucket> = std::collections::BTreeMap::new();
    let mut t = today_aligned;
    while t <= now_aligned {
        let label = chrono::DateTime::from_timestamp(t, 0)
            .map(|d| d.with_timezone(&local_tz))
            .map(|d| d.format("%H:%M").to_string())
            .unwrap_or_else(|| "00:00".to_string());
        buckets.insert(
            label,
            Bucket {
                api_calls: 0,
                tokens: 0,
            },
        );
        t += 600;
    }

    let rows = query_rows_by_model(
        conn,
        today_start,
        model,
        "SELECT timestamp, api_calls, prompt_tokens + completion_tokens
         FROM api_usage_logs WHERE timestamp >= ?1",
        "SELECT timestamp, api_calls, prompt_tokens + completion_tokens
         FROM api_usage_logs WHERE timestamp >= ?1 AND model = ?2",
    )?;

    for (ts, cnt, tok) in rows {
        let bucket_ts = (ts as i64 / 600) * 600;
        let label = chrono::DateTime::from_timestamp(bucket_ts, 0)
            .map(|d| d.with_timezone(&local_tz))
            .map(|d| d.format("%H:%M").to_string())
            .unwrap_or_else(|| "00:00".to_string());
        if let Some(b) = buckets.get_mut(&label) {
            b.api_calls += cnt;
            b.tokens += tok;
        }
    }

    Ok(buckets)
}

/// Query rolling hourly buckets (past 24 hours).
pub fn query_hourly_rolling_24h(
    conn: &Connection,
    start_ts: f64,
    now: f64,
    model: Option<&str>,
) -> rusqlite::Result<std::collections::BTreeMap<String, crate::models::types::Bucket>> {
    use crate::models::types::Bucket;
    #[allow(unused_imports)]
    use chrono::TimeZone;
    let local_tz = chrono::Local;

    let current_hour_ts = (now as i64 / 3600) * 3600;
    let start_hour_ts = (start_ts as i64 / 3600) * 3600;

    let mut hourly: std::collections::BTreeMap<String, Bucket> = std::collections::BTreeMap::new();
    let mut t = start_hour_ts;
    while t <= current_hour_ts {
        let label = chrono::DateTime::from_timestamp(t, 0)
            .map(|d| d.with_timezone(&local_tz))
            .map(|d| d.format("%H:00").to_string())
            .unwrap_or_else(|| "00:00".to_string());
        hourly.insert(label, Bucket { api_calls: 0, tokens: 0 });
        t += 3600;
    }

    let rows = query_rows_by_model(
        conn,
        start_ts,
        model,
        "SELECT timestamp, api_calls, prompt_tokens + completion_tokens
         FROM api_usage_logs WHERE timestamp >= ?1",
        "SELECT timestamp, api_calls, prompt_tokens + completion_tokens
         FROM api_usage_logs WHERE timestamp >= ?1 AND model = ?2",
    )?;

    for (ts, cnt, tok) in rows {
        let h = chrono::DateTime::from_timestamp(ts as i64, 0)
            .map(|d| d.with_timezone(&local_tz))
            .map(|d| d.format("%H:00").to_string())
            .unwrap_or_else(|| "00:00".to_string());
        if let Some(b) = hourly.get_mut(&h) {
            b.api_calls += cnt;
            b.tokens += tok;
        }
    }

    Ok(hourly)
}

/// Get unique models from usage.db
pub fn get_models(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT model FROM api_usage_logs
         WHERE model IS NOT NULL AND model != ''
         ORDER BY model"
    )?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut models = Vec::new();
    for row in rows {
        models.push(row?);
    }
    Ok(models)
}

/// Get model summaries (all-time)
pub fn get_model_summaries(
    conn_usage: &Connection,
    conn_state: &rusqlite::Connection,
) -> rusqlite::Result<std::collections::BTreeMap<String, crate::models::types::ModelSummary>> {
    use crate::models::types::ModelSummary;
    let models = get_models(conn_usage)?;
    let mut summaries = std::collections::BTreeMap::new();

    for m in &models {
        let (api_calls, input_tokens, output_tokens, sessions) = conn_usage.query_row(
            "SELECT
                COALESCE(SUM(api_calls), 0),
                COALESCE(SUM(prompt_tokens), 0),
                COALESCE(SUM(completion_tokens), 0),
                COALESCE(COUNT(DISTINCT session_id), 0)
             FROM api_usage_logs
             WHERE model = ?1",
            rusqlite::params![m],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )?;

        let (cache_read, cache_write, reasoning) = conn_state.query_row(
            "SELECT
                COALESCE(SUM(cache_read_tokens), 0),
                COALESCE(SUM(cache_write_tokens), 0),
                COALESCE(SUM(reasoning_tokens), 0)
             FROM sessions
             WHERE model = ?1",
            rusqlite::params![m],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )?;

        let total_tokens = input_tokens + output_tokens + cache_read + cache_write + reasoning;

        summaries.insert(m.clone(), ModelSummary {
            api_calls,
            input_tokens,
            output_tokens,
            cache_read_tokens: cache_read,
            cache_write_tokens: cache_write,
            reasoning_tokens: reasoning,
            sessions,
            total_tokens,
        });
    }

    Ok(summaries)
}
