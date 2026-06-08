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

/// Strip reasoning-model `<think>...</think>` blocks if any leak through.
fn strip_think(s: &str) -> String {
    let mut out = s.to_string();
    while let (Some(a), Some(b)) = (out.find("<think>"), out.find("</think>")) {
        if b > a {
            out.replace_range(a..b + "</think>".len(), "");
        } else {
            break;
        }
    }
    out.trim().to_string()
}

/// Best-effort: pull the first {...} JSON object out of a noisy string.
fn extract_json(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    if end > start {
        Some(&s[start..=end])
    } else {
        None
    }
}

/// Ask Ollama in JSON mode and deserialize into `T`. Returns None on any failure.
pub async fn generate_json<T: DeserializeOwned>(prompt: &str) -> Option<T> {
    let body = json!({
        "model": config::ollama_model(),
        "prompt": prompt,
        "stream": false,
        "format": "json",
        "options": { "temperature": 0.1, "num_ctx": 8192 }
    });
    let resp = client(240)
        .post(format!("{}/api/generate", config::OLLAMA_URL))
        .json(&body)
        .send()
        .await
        .ok()?;
    let v: serde_json::Value = resp.json().await.ok()?;
    let raw = strip_think(v.get("response")?.as_str()?);
    if raw.is_empty() {
        return None;
    }
    serde_json::from_str::<T>(&raw)
        .ok()
        .or_else(|| extract_json(&raw).and_then(|j| serde_json::from_str::<T>(j).ok()))
}

/// Free-form text generation. Returns empty string on failure.
pub async fn generate_text(prompt: &str) -> String {
    let body = json!({
        "model": config::ollama_model(),
        "prompt": prompt,
        "stream": false,
        "options": { "temperature": 0.2, "num_ctx": 8192 }
    });
    let out = async {
        let resp = client(240)
            .post(format!("{}/api/generate", config::OLLAMA_URL))
            .json(&body)
            .send()
            .await
            .ok()?;
        let v: serde_json::Value = resp.json().await.ok()?;
        Some(strip_think(v.get("response")?.as_str()?))
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
