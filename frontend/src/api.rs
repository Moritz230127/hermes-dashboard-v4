use crate::types::ApiDataResponse;
use wasm_bindgen::prelude::*;

fn log(msg: &str) {
    web_sys::console::log_1(&JsValue::from_str(msg));
}

pub async fn fetch_data(model: String) -> Option<ApiDataResponse> {
    let url = format!("/api/data?model={}", model);
    match gloo_net::http::Request::get(&url).send().await {
        Ok(resp) => match resp.json::<ApiDataResponse>().await {
            Ok(data) => Some(data),
            Err(e) => {
                log(&format!("JSON parse error: {}", e));
                None
            }
        },
        Err(e) => {
            log(&format!("HTTP error: {}", e));
            None
        }
    }
}
