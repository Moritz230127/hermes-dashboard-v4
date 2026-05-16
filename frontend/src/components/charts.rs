use dioxus::prelude::*;
use crate::types::ApiDataResponse;

#[component]
pub fn ChartPanel(data: Option<ApiDataResponse>) -> Element {
    rsx! {
        div { class: "chart-panel",
            h3 { "24h Hourly Breakdown" }
            canvas { id: "chart-hourly-24h", class: "dashboard-chart" }
            h3 { "10-Minute Buckets" }
            canvas { id: "chart-10min", class: "dashboard-chart" }
            h3 { "7-Day Daily" }
            canvas { id: "chart-7d", class: "dashboard-chart" }
        }
    }
}
