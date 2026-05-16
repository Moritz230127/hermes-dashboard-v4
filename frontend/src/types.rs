use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
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
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Bucket {
    pub api_calls: i64,
    pub tokens: i64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct HistoricalData {
    pub today: PeriodStats,
    pub yesterday: PeriodStats,
    pub avg_7d: PeriodStats,
    pub daily_breakdown: std::collections::BTreeMap<String, DailyItem>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct DailyItem {
    pub date: String,
    pub data: PeriodStats,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub time: String,
    pub date: String,
    pub model: String,
    pub api_calls: i64,
    pub messages: i64,
    pub tool_calls: i64,
    pub total_tokens: i64,
    pub duration_s: Option<f64>,
    pub tps: Option<f64>,
    pub cost: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ModelSummary {
    pub api_calls: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub total_tokens: i64,
    pub sessions: i64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Alert {
    pub level: String,
    pub category: String,
    pub message: String,
    pub value: Option<f64>,
    pub threshold: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct DashboardConfig {
    pub today_start: String,
    pub timezone: String,
    pub thresholds: AlertThresholds,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AlertThresholds {
    pub min_tps: f64,
    pub max_premature_rate: f64,
    pub min_cache_hit_rate: f64,
    pub max_api_spike_ratio: f64,
}
