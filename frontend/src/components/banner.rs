use dioxus::prelude::*;

#[component]
pub fn Banner() -> Element {
    rsx! {
        div { class: "banner",
            h1 { "Hermes Agent Dashboard" }
            div { class: "subtitle", "Real-time usage monitoring & analytics" }
        }
    }
}
