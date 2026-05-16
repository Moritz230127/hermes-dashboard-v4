use dioxus::prelude::*;
use crate::types::{ApiDataResponse, PeriodStats};

fn format_num(n: i64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn format_cost(cost: f64) -> String {
    if cost >= 1.0 {
        format!("${:.2}", cost)
    } else if cost > 0.0 {
        format!("{:.2}¢", cost * 100.0)
    } else {
        "$0.00".to_string()
    }
}

#[component]
pub fn StatCards(data: Option<ApiDataResponse>) -> Element {
    // Default stats when data is loading
    let default_stats = PeriodStats {
        api_calls: 0, total_tokens: 0, input_tokens: 0, output_tokens: 0,
        sessions: 0, messages: 0, tool_calls: 0, est_cost: 0.0, avg_tps: 0.0,
        avg_duration_s: 0.0, avg_context_usage: 0.0, cache_read_tokens: 0,
        cache_write_tokens: 0, reasoning_tokens: 0, act_cost: 0.0, premature: 0,
        completed_sessions: 0, avg_llm_calls_per_session: 0.0, avg_output_tokens: 0.0,
    };

    let s = data.as_ref().map(|d| &d.today).unwrap_or(&default_stats);
    let r5 = data.as_ref().map(|d| &d.rolling_5h).unwrap_or(&default_stats);

    rsx! {
        div { class: "stats-grid",
            div { class: "stat-card", style: "border-left: 3px solid #4CAF50",
                div { class: "stat-value", "{format_num(s.api_calls)}" }
                div { class: "stat-label", "Today API Calls" }
            }
            div { class: "stat-card", style: "border-left: 3px solid #2196F3",
                div { class: "stat-value", "{format_num(s.total_tokens)}" }
                div { class: "stat-label", "Today Total Tokens" }
            }
            div { class: "stat-card", style: "border-left: 3px solid #FF9800",
                div { class: "stat-value", "{format_num(s.sessions)}" }
                div { class: "stat-label", "Today Sessions" }
            }
            div { class: "stat-card", style: "border-left: 3px solid #9C27B0",
                div { class: "stat-value", "{format_cost(s.est_cost)}" }
                div { class: "stat-label", "Today Cost" }
            }
            div { class: "stat-card", style: "border-left: 3px solid #00BCD4",
                div { class: "stat-value", "{format_num(r5.api_calls)}" }
                div { class: "stat-label", "5h API Calls" }
            }
            div { class: "stat-card", style: "border-left: 3px solid #E91E63",
                div { class: "stat-value", "{format_num(r5.total_tokens)}" }
                div { class: "stat-label", "5h Tokens" }
            }
        }
    }
}
