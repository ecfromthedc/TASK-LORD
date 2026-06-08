//! Local LLM client (Ollama HTTP API). The token-grind lives here.

use crate::config;
use serde::de::DeserializeOwned;
use serde_json::json;
use std::time::Duration;

fn client(timeout_secs: u64) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .expect("reqwest client")
}

/// Ask Ollama in JSON mode and deserialize into `T`. Returns None on any failure.
pub async fn generate_json<T: DeserializeOwned>(prompt: &str) -> Option<T> {
    let body = json!({
        "model": config::OLLAMA_MODEL,
        "prompt": prompt,
        "stream": false,
        "format": "json",
        "options": { "temperature": 0.1, "num_ctx": 8192 }
    });
    let resp = client(180)
        .post(format!("{}/api/generate", config::OLLAMA_URL))
        .json(&body)
        .send()
        .await
        .ok()?;
    let v: serde_json::Value = resp.json().await.ok()?;
    let raw = v.get("response")?.as_str()?.trim().to_string();
    if raw.is_empty() {
        return None;
    }
    serde_json::from_str::<T>(&raw).ok()
}

/// Free-form text generation. Returns empty string on failure.
pub async fn generate_text(prompt: &str) -> String {
    let body = json!({
        "model": config::OLLAMA_MODEL,
        "prompt": prompt,
        "stream": false,
        "options": { "temperature": 0.2, "num_ctx": 8192 }
    });
    let out = async {
        let resp = client(180)
            .post(format!("{}/api/generate", config::OLLAMA_URL))
            .json(&body)
            .send()
            .await
            .ok()?;
        let v: serde_json::Value = resp.json().await.ok()?;
        Some(v.get("response")?.as_str()?.trim().to_string())
    }
    .await;
    out.unwrap_or_default()
}

pub async fn is_up() -> bool {
    client(5)
        .get(format!("{}/api/tags", config::OLLAMA_URL))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}
