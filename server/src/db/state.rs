use rusqlite::Connection;
use std::path::Path;

use crate::models::types::{AuxStats, RunningSession, SessionInfo};

/// Open state.db connection
pub fn open(db_path: &Path) -> rusqlite::Result<Connection> {
    Connection::open(db_path)
}

/// Query auxiliary metrics for a time period.
pub fn query_period_aux(
    conn: &Connection,
    cutoff_start: f64,
    cutoff_end: Option<f64>,
) -> rusqlite::Result<AuxStats> {
    let (messages, tool_calls, cache_read, cache_write, reasoning, est_cost, act_cost, premature) =
        match cutoff_end {
            Some(end) => conn.query_row(
                "SELECT
                    COALESCE(SUM(message_count), 0),
                    COALESCE(SUM(tool_call_count), 0),
                    COALESCE(SUM(cache_read_tokens), 0),
                    COALESCE(SUM(cache_write_tokens), 0),
                    COALESCE(SUM(reasoning_tokens), 0),
                    COALESCE(SUM(estimated_cost_usd), 0),
                    COALESCE(SUM(COALESCE(actual_cost_usd, estimated_cost_usd, 0)), 0),
                    COALESCE(SUM(CASE WHEN end_reason = 'new_session' THEN 1 ELSE 0 END), 0)
                 FROM sessions
                 WHERE started_at >= ?1 AND started_at < ?2",
                rusqlite::params![cutoff_start, end],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, f64>(5)?,
                        row.get::<_, f64>(6)?,
                        row.get::<_, i64>(7)?,
                    ))
                },
            )?,
            None => conn.query_row(
                "SELECT
                    COALESCE(SUM(message_count), 0),
                    COALESCE(SUM(tool_call_count), 0),
                    COALESCE(SUM(cache_read_tokens), 0),
                    COALESCE(SUM(cache_write_tokens), 0),
                    COALESCE(SUM(reasoning_tokens), 0),
                    COALESCE(SUM(estimated_cost_usd), 0),
                    COALESCE(SUM(COALESCE(actual_cost_usd, estimated_cost_usd, 0)), 0),
                    COALESCE(SUM(CASE WHEN end_reason = 'new_session' THEN 1 ELSE 0 END), 0)
                 FROM sessions
                 WHERE started_at >= ?1",
                rusqlite::params![cutoff_start],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, f64>(5)?,
                        row.get::<_, f64>(6)?,
                        row.get::<_, i64>(7)?,
                    ))
                },
            )?,
        };

    // Completed sessions for performance metrics
    let completed = match cutoff_end {
        Some(end) => {
            let mut stmt = conn.prepare(
                "SELECT started_at, ended_at, api_call_count,
                        input_tokens, output_tokens, cache_read_tokens
                 FROM sessions
                 WHERE started_at >= ?1 AND started_at < ?2
                   AND ended_at IS NOT NULL AND api_call_count > 0"
            )?;
            stmt.query_map(rusqlite::params![cutoff_start, end], |row| {
                Ok((
                    row.get::<_, f64>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            })?.collect::<Result<Vec<_>, _>>()?
        }
        None => {
            let mut stmt = conn.prepare(
                "SELECT started_at, ended_at, api_call_count,
                        input_tokens, output_tokens, cache_read_tokens
                 FROM sessions
                 WHERE started_at >= ?1
                   AND ended_at IS NOT NULL AND api_call_count > 0"
            )?;
            stmt.query_map(rusqlite::params![cutoff_start], |row| {
                Ok((
                    row.get::<_, f64>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            })?.collect::<Result<Vec<_>, _>>()?
        }
    };

    let completed_sessions = completed.len() as i64;

    let mut durations: Vec<f64> = Vec::new();
    let mut total_output = 0i64;
    let mut total_input = 0i64;

    for (start, end, _calls, inp, out, _cache) in &completed {
        if *end > *start {
            let dur = end - start;
            durations.push(dur);
            total_output += out;
            total_input += inp;
        }
    }

    let total_dur: f64 = durations.iter().sum();
    let avg_duration_s = if !durations.is_empty() {
        total_dur / durations.len() as f64
    } else {
        0.0
    };

    let avg_tps = if total_dur > 0.0 {
        total_output as f64 / total_dur
    } else {
        0.0
    };

    let avg_context_usage = if completed_sessions > 0 {
        total_input as f64 / completed_sessions as f64
    } else {
        0.0
    };

    Ok(AuxStats {
        messages,
        tool_calls,
        cache_read_tokens: cache_read,
        cache_write_tokens: cache_write,
        reasoning_tokens: reasoning,
        est_cost,
        act_cost,
        premature,
        completed_sessions,
        avg_duration_s,
        avg_tps,
        avg_context_usage,
    })
}

/// Get recent sessions.
pub fn get_sessions(
    conn: &Connection,
    cutoff: f64,
    model: Option<&str>,
    limit: i64,
) -> rusqlite::Result<Vec<SessionInfo>> {
    let rows = if model.is_none_or(|m| m == "__all__") {
        let mut stmt = conn.prepare(
            "SELECT id, started_at, model, api_call_count, message_count,
                    tool_call_count, input_tokens, output_tokens, cache_read_tokens,
                    end_reason, ended_at, estimated_cost_usd,
                    COALESCE(actual_cost_usd, estimated_cost_usd, 0),
                    COALESCE(title, '')
             FROM sessions
             WHERE started_at > ?1 AND api_call_count > 0
             ORDER BY started_at DESC
             LIMIT ?2"
        )?;
        stmt.query_map(rusqlite::params![cutoff, limit], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, f64>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, i64>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, Option<f64>>(10)?,
                row.get::<_, f64>(11)?,
                row.get::<_, f64>(12)?,
                row.get::<_, String>(13)?,
            ))
        })?.collect::<Result<Vec<_>, _>>()?
    } else {
        let mut stmt = conn.prepare(
            "SELECT id, started_at, model, api_call_count, message_count,
                    tool_call_count, input_tokens, output_tokens, cache_read_tokens,
                    end_reason, ended_at, estimated_cost_usd,
                    COALESCE(actual_cost_usd, estimated_cost_usd, 0),
                    COALESCE(title, '')
             FROM sessions
             WHERE started_at > ?1 AND api_call_count > 0 AND model = ?2
             ORDER BY started_at DESC
             LIMIT ?3"
        )?;
        stmt.query_map(rusqlite::params![cutoff, model.unwrap(), limit], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, f64>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, i64>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, Option<f64>>(10)?,
                row.get::<_, f64>(11)?,
                row.get::<_, f64>(12)?,
                row.get::<_, String>(13)?,
            ))
        })?.collect::<Result<Vec<_>, _>>()?
    };

    let sessions: Vec<SessionInfo> = rows
        .into_iter()
        .map(|r| {
            let (
                id,
                started_at,
                model,
                api_call_count,
                message_count,
                tool_call_count,
                input_tokens,
                output_tokens,
                cache_read_tokens,
                end_reason,
                ended_at,
                _est_cost,
                cost,
                title,
            ) = r;

            let dur = ended_at.map(|end| end - started_at);
            let tps = dur.map(|d| {
                if d > 0.0 {
                    output_tokens as f64 / d
                } else {
                    0.0
                }
            });

            let dt = chrono::DateTime::from_timestamp(started_at as i64, 0)
                .unwrap_or_default()
                .with_timezone(&chrono::Local);

            SessionInfo {
                id,
                time: dt.format("%H:%M").to_string(),
                date: dt.format("%m-%d").to_string(),
                model: model.unwrap_or_else(|| "unknown".to_string()),
                title,
                api_calls: api_call_count,
                messages: message_count,
                tool_calls: tool_call_count,
                input_tokens,
                output_tokens,
                cache_read_tokens,
                total_tokens: input_tokens + output_tokens + cache_read_tokens,
                end_reason,
                duration_s: dur,
                tps,
                cost,
            }
        })
        .collect();

    Ok(sessions)
}

/// Get running sessions.
pub fn get_running_sessions(conn: &Connection) -> rusqlite::Result<Vec<RunningSession>> {
    let mut stmt = conn.prepare(
        "SELECT id, started_at, model, api_call_count,
                message_count, tool_call_count, title
         FROM sessions
         WHERE ended_at IS NULL
         ORDER BY started_at DESC"
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, f64>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, Option<String>>(6)?,
        ))
    })?;

    let mut sessions = Vec::new();
    for row in rows {
        let (id, started_at, model, api_call_count,
             message_count, tool_call_count, title) = row?;

        let dt = chrono::DateTime::from_timestamp(started_at as i64, 0)
            .unwrap_or_default();

        sessions.push(RunningSession {
            id,
            started_at,
            started_at_str: dt.format("%Y-%m-%d %H:%M:%S").to_string(),
            model: model.unwrap_or_else(|| "unknown".to_string()),
            api_call_count,
            message_count,
            tool_call_count,
            title: title.unwrap_or_else(|| "Untitled".to_string()),
            is_running: true,
        });
    }

    Ok(sessions)
}

/// Stop a specific session.
pub fn stop_session(conn: &Connection, session_id: &str, now: f64) -> rusqlite::Result<usize> {
    let updated = conn.execute(
        "UPDATE sessions SET ended_at = ?1, end_reason = 'dashboard_stopped' WHERE id = ?2",
        rusqlite::params![now, session_id],
    )?;
    Ok(updated)
}

/// Stop all running sessions except the most recent one.
pub fn stop_other_sessions(
    conn: &Connection,
    now: f64,
) -> rusqlite::Result<(i64, Option<String>)> {
    let latest: Option<String> = conn
        .query_row(
            "SELECT id FROM sessions
             WHERE ended_at IS NULL
             ORDER BY started_at DESC
             LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok();

    let stopped = match &latest {
        Some(latest_id) => conn.execute(
            "UPDATE sessions
             SET ended_at = ?1, end_reason = 'dashboard_stopped_others'
             WHERE ended_at IS NULL AND id != ?2",
            rusqlite::params![now, latest_id],
        )? as i64,
        None => 0,
    };

    Ok((stopped, latest))
}

/// Mark old sessions for compression.
pub fn mark_compression(conn: &Connection, cutoff: f64, session_id: Option<&str>) -> rusqlite::Result<i64> {
    let count = match session_id {
        Some(sid) => conn.execute(
            "UPDATE sessions
             SET end_reason = 'compression_marked'
             WHERE id = ?1 AND ended_at IS NOT NULL",
            rusqlite::params![sid],
        )? as i64,
        None => conn.execute(
            "UPDATE sessions
             SET end_reason = 'compression_marked'
             WHERE ended_at IS NOT NULL AND ended_at < ?1 AND end_reason != 'compression'",
            rusqlite::params![cutoff],
        )? as i64,
    };
    Ok(count)
}

/// Count old ended sessions for cleanup report.
pub fn count_old_sessions(conn: &Connection, cutoff: f64) -> rusqlite::Result<i64> {
    let count = conn.query_row(
        "SELECT COUNT(*) FROM sessions
         WHERE ended_at IS NOT NULL AND ended_at < ?1",
        rusqlite::params![cutoff],
        |row| row.get::<_, i64>(0),
    )?;
    Ok(count)
}

/// ACTUALLY DELETE old ended sessions and VACUUM the database.
/// This is a REAL memory cleanup (unlike the fake one in 8653 that just counted).
/// Returns (deleted_count, vacuum_size_saved_approx)
pub fn cleanup_old_sessions(conn: &mut Connection, cutoff: f64) -> rusqlite::Result<(i64, i64)> {
    // Count before deletion
    let count = count_old_sessions(conn, cutoff)?;

    // Get DB size before
    let before_size: i64 = conn.query_row(
        "SELECT COALESCE(SUM(pgsize), 0) FROM dbstat WHERE name = 'sessions'",
        [],
        |row| row.get::<_, i64>(0),
    ).unwrap_or(0);

    // Delete old ended sessions
    conn.execute(
        "DELETE FROM sessions
         WHERE ended_at IS NOT NULL AND ended_at < ?1",
        rusqlite::params![cutoff],
    )?;

    // Also delete associated usage logs for cleaned sessions
    // (We can't easily cross-ref, so we keep usage logs - they're append-only and small)
    
    // VACUUM to reclaim space
    conn.execute("VACUUM", [])?;

    // Get size after
    let after_size: i64 = conn.query_row(
        "SELECT COALESCE(SUM(pgsize), 0) FROM dbstat WHERE name = 'sessions'",
        [],
        |row| row.get::<_, i64>(0),
    ).unwrap_or(0);

    let saved = if before_size > after_size { before_size - after_size } else { 0 };

    Ok((count, saved))
}
