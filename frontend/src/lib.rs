mod api;
mod components;
mod types;

use dioxus::prelude::*;
use wasm_bindgen::prelude::*;
use crate::types::ApiDataResponse;

#[wasm_bindgen(start)]
pub fn start() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let selected_model = use_signal(|| "__all__".to_string());
    let resource = use_resource(move || async move {
        api::fetch_data(selected_model()).await
    });

    // resource.read() returns Option<Option<ApiDataResponse>>; flatten once
    let data: Option<ApiDataResponse> = resource.read().as_ref().and_then(|d| d.clone());

    rsx! {
        div { class: "app",
            components::banner::Banner {}
            div { class: "content",
                div { class: "sidebar",
                    components::stats::StatCards { data: data.clone() }
                }
                div { class: "main-area",
                    components::charts::ChartPanel { data: data.clone() }
                    components::sessions::RecentSessions { data }
                }
            }
        }
    }
}
