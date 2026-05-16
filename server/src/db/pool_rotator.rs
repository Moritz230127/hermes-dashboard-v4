use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
struct KeyState {
    credential_id: String,
    label: String,
    timestamps: Vec<f64>,
    cooling_until: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
struct PoolState {
    states: BTreeMap<String, KeyState>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PoolKeyStats {
    pub credential_id: String,
    pub label: String,
    pub request_count: usize,
    pub is_cooling: bool,
    pub cooling_until: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PoolRotatorStatus {
    pub available: bool,
    pub total_keys: usize,
    pub total_requests: usize,
    pub keys: Vec<PoolKeyStats>,
}

/// Read pool rotator state from state.json
pub fn read_pool_state(path: &Path) -> PoolRotatorStatus {
    if !path.exists() {
        return PoolRotatorStatus {
            available: false,
            total_keys: 0,
            total_requests: 0,
            keys: Vec::new(),
        };
    }

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return PoolRotatorStatus {
            available: false,
            total_keys: 0,
            total_requests: 0,
            keys: Vec::new(),
        },
    };

    let pool: PoolState = match serde_json::from_str(&content) {
        Ok(p) => p,
        Err(_) => return PoolRotatorStatus {
            available: false,
            total_keys: 0,
            total_requests: 0,
            keys: Vec::new(),
        },
    };

    let keys: Vec<PoolKeyStats> = pool.states.values().map(|k| {
        let request_count = k.timestamps.len();
        let is_cooling = k.cooling_until.map(|until| until > 0.0).unwrap_or(false);
        PoolKeyStats {
            credential_id: k.credential_id.clone(),
            label: k.label.clone(),
            request_count,
            is_cooling,
            cooling_until: k.cooling_until,
        }
    }).collect();

    let total_requests: usize = keys.iter().map(|k| k.request_count).sum();

    PoolRotatorStatus {
        available: true,
        total_keys: keys.len(),
        total_requests,
        keys,
    }
}
