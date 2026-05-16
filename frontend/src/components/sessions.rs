use dioxus::prelude::*;
use crate::types::ApiDataResponse;

#[component]
pub fn RecentSessions(data: Option<ApiDataResponse>) -> Element {
    // Pre-compute session references
    let session_refs: Vec<&crate::types::SessionInfo> = match &data {
        Some(d) => d.sessions.iter().take(20).collect(),
        None => vec![],
    };

    rsx! {
        div { class: "sessions-table",
            h3 { "Recent Sessions" }
            table {
                thead {
                    tr {
                        th { "Time" }
                        th { "Model" }
                        th { "API Calls" }
                        th { "Tokens" }
                        th { "Duration" }
                        th { "TPS" }
                    }
                }
                tbody {
                    {session_refs.into_iter().map(|session| {
                        let dur_str = session.duration_s
                            .map(|d| format!("{:.0}s", d))
                            .unwrap_or_else(|| "-".to_string());
                        let tps_str = session.tps
                            .map(|t| format!("{:.1}", t))
                            .unwrap_or_else(|| "-".to_string());
                        rsx! {
                            tr {
                                td { "{session.time}" }
                                td { "{session.model}" }
                                td { "{session.api_calls}" }
                                td { "{session.total_tokens}" }
                                td { "{dur_str}" }
                                td { "{tps_str}" }
                            }
                        }
                    })}
                }
            }
        }
    }
}
