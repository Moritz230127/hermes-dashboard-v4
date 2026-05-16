use serde::Serialize;

// ============================================================================
// Core period statistics
// ============================================================================

#[derive(Debug, Clone, Serialize, Default)]
pub struct PeriodStats {
    pub api_calls: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub sessions: i64,
    pub messages: i64,
    pub tool_calls: i64,
    pub cache_read_tokens: i64,
    pub cache_write_tokens: i64,
    pub reasoning_tokens: i64,
    pub est_cost: f64,
    pub act_cost: f64,
    pub premature: i64,
    pub completed_sessions: i64,
    pub avg_duration_s: f64,
    pub avg_tps: f64,
    pub avg_context_usage: f64,
    pub total_tokens: i64,
    pub avg_llm_calls_per_session: f64,
    pub avg_output_tokens: f64,
}

impl PeriodStats {
    pub fn compute(core: &CoreStats, aux: &AuxStats) -> Self {
        let total_tokens = core.input_tokens
            + core.output_tokens
            + aux.cache_read_tokens
            + aux.cache_write_tokens
            + aux.reasoning_tokens;

        let sessions = core.sessions;
        let api_calls = core.api_calls;

        let avg_llm_calls = if sessions > 0 {
            api_calls as f64 / sessions as f64
        } else {
            0.0
        };

        let avg_output = if sessions > 0 {
            core.output_tokens as f64 / sessions as f64
        } else {
            0.0
        };

        Self {
            api_calls: core.api_calls,
            input_tokens: core.input_tokens,
            output_tokens: core.output_tokens,
            sessions: core.sessions,
            messages: aux.messages,
            tool_calls: aux.tool_calls,
            cache_read_tokens: aux.cache_read_tokens,
            cache_write_tokens: aux.cache_write_tokens,
            reasoning_tokens: aux.reasoning_tokens,
            est_cost: aux.est_cost,
            act_cost: aux.act_cost,
            premature: aux.premature,
            completed_sessions: aux.completed_sessions,
            avg_duration_s: aux.avg_duration_s,
            avg_tps: aux.avg_tps,
            avg_context_usage: aux.avg_context_usage,
            total_tokens,
            avg_llm_calls_per_session: avg_llm_calls,
            avg_output_tokens: avg_output,
        }
    }
}

// ============================================================================
// Core stats from usage.db
// ============================================================================

#[derive(Debug, Clone, Default)]
pub struct CoreStats {
    pub api_calls: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub sessions: i64,
}

// ============================================================================
// Auxiliary stats from state.db
// ============================================================================

#[derive(Debug, Clone, Default)]
pub struct AuxStats {
    pub messages: i64,
    pub tool_calls: i64,
    pub cache_read_tokens: i64,
    pub cache_write_tokens: i64,
    pub reasoning_tokens: i64,
    pub est_cost: f64,
    pub act_cost: f64,
    pub premature: i64,
    pub completed_sessions: i64,
    pub avg_duration_s: f64,
    pub avg_tps: f64,
    pub avg_context_usage: f64,
}

// ============================================================================
// Hourly / 10min bucket
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct Bucket {
    pub api_calls: i64,
    pub tokens: i64,
}

// ============================================================================
// Daily breakdown item
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct DailyItem {
    pub date: String,
    pub data: PeriodStats,
}

// ============================================================================
// Historical data
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct HistoricalData {
    pub today: PeriodStats,
    pub yesterday: PeriodStats,
    pub day_before_yesterday: PeriodStats,
    pub avg_7d: PeriodStats,
    pub daily_breakdown: std::collections::BTreeMap<String, DailyItem>,
    pub daily_30: Vec<DailyItem>,
}

// ============================================================================
// Session info
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct SessionInfo {
    pub id: String,
    pub time: String,
    pub date: String,
    pub model: String,
    pub title: String,
    pub api_calls: i64,
    pub messages: i64,
    pub tool_calls: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub total_tokens: i64,
    pub end_reason: Option<String>,
    pub duration_s: Option<f64>,
    pub tps: Option<f64>,
    pub cost: f64,
}

// ============================================================================
// Running session
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct RunningSession {
    pub id: String,
    pub started_at: f64,
    pub started_at_str: String,
    pub model: String,
    pub api_call_count: i64,
    pub message_count: i64,
    pub tool_call_count: i64,
    pub title: String,
    pub is_running: bool,
}

// ============================================================================
// Model summary
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct ModelSummary {
    pub api_calls: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_write_tokens: i64,
    pub reasoning_tokens: i64,
    pub sessions: i64,
    pub total_tokens: i64,
}

// ============================================================================
// Alert
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct Alert {
    pub level: String,
    pub category: String,
    pub message: String,
    pub value: Option<f64>,
    pub threshold: Option<f64>,
}

use crate::db::pool_rotator::PoolRotatorStatus;

// ============================================================================
// Full API response
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct ApiDataResponse {
    pub models: Vec<String>,
    pub today: PeriodStats,
    pub historical: HistoricalData,
    pub hourly: std::collections::BTreeMap<String, Bucket>,
    pub buckets_10m: std::collections::BTreeMap<String, Bucket>,
    pub sessions: Vec<SessionInfo>,
    pub model_summaries: std::collections::BTreeMap<String, ModelSummary>,
    pub alerts: Vec<Alert>,
    pub now: f64,
    pub context_window: i64,
    pub rolling_5h: PeriodStats,
    pub rolling_24h: PeriodStats,
    pub hourly_rolling_24h: std::collections::BTreeMap<String, Bucket>,
    pub config: DashboardConfig,
    pub pool_rotator: PoolRotatorStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct DashboardConfig {
    pub today_start: String,
    pub timezone: String,
    pub thresholds: AlertThresholds,
}

#[derive(Debug, Clone, Serialize)]
pub struct AlertThresholds {
    pub min_tps: f64,
    pub max_premature_rate: f64,
    pub min_cache_hit_rate: f64,
    pub max_api_spike_ratio: f64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            min_tps: 5.0,
            max_premature_rate: 15.0,
            min_cache_hit_rate: 30.0,
            max_api_spike_ratio: 3.0,
        }
    }
}

// ============================================================================
// Health response
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: f64,
    pub version: String,
}

// ============================================================================
// Generic status response
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct StatusResponse {
    pub status: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stopped_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kept_session: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_sessions_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

// ============================================================================
// Running sessions response
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct RunningSessionsResponse {
    pub count: usize,
    pub sessions: Vec<RunningSession>,
}
