//! Hosted DeepSeek API client (OpenAI-compatible). Higher-quality summaries
//! than the local model when a key is configured. The key is read at call time
//! and never logged.

use crate::config;
use serde::de::DeserializeOwned;
use serde_json::json;
use std::time::Duration;

fn client(secs: u64) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(secs))
        .build()
        .expect("reqwest client")
}

fn extract_json(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    (end > start).then(|| &s[start..=end])
}

async fn chat(prompt: &str, json_mode: bool, secs: u64) -> Option<String> {
    let key = config::deepseek_key()?;
    let mut body = json!({
        "model": config::deepseek_model(),
        "messages": [{"role": "user", "content": prompt}],
        "stream": false,
        "temperature": 0.2
    });
    if json_mode {
        body["response_format"] = json!({"type": "json_object"});
        body["temperature"] = json!(0.1);
    }
    let resp = client(secs)
        .post(config::DEEPSEEK_URL)
        .bearer_auth(key)
        .json(&body)
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        eprintln!("[deepseek] HTTP {}", resp.status());
        return None;
    }
    let v: serde_json::Value = resp.json().await.ok()?;
    Some(v.get("choices")?.get(0)?.get("message")?.get("content")?.as_str()?.trim().to_string())
}

pub async fn generate_json<T: DeserializeOwned>(prompt: &str) -> Option<T> {
    let raw = chat(prompt, true, 120).await?;
    serde_json::from_str::<T>(&raw)
        .ok()
        .or_else(|| extract_json(&raw).and_then(|j| serde_json::from_str::<T>(j).ok()))
}

pub async fn generate_text(prompt: &str) -> String {
    chat(prompt, false, 120).await.unwrap_or_default()
}

/// A cheap auth/connectivity probe.
pub async fn available() -> bool {
    config::deepseek_key().is_some()
}
