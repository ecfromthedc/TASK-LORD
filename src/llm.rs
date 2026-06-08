//! Provider dispatch: route summaries/handoffs to the hosted DeepSeek API or
//! the local Ollama model based on configuration. Falls back gracefully.

use crate::{config, deepseek, ollama};
use serde::de::DeserializeOwned;

fn use_deepseek() -> bool {
    config::provider() == "deepseek" && config::deepseek_key().is_some()
}

/// True if the selected backend is reachable/usable.
pub async fn available() -> bool {
    if use_deepseek() {
        deepseek::available().await
    } else {
        ollama::is_up().await
    }
}

/// Human-readable description of the active backend (for logs; no secrets).
pub fn describe() -> String {
    if use_deepseek() {
        format!("deepseek API ({})", config::deepseek_model())
    } else {
        format!("ollama ({})", config::ollama_model())
    }
}

pub async fn generate_json<T: DeserializeOwned>(prompt: &str) -> Option<T> {
    if use_deepseek() {
        deepseek::generate_json(prompt).await
    } else {
        ollama::generate_json(prompt).await
    }
}

pub async fn generate_text(prompt: &str) -> String {
    if use_deepseek() {
        deepseek::generate_text(prompt).await
    } else {
        ollama::generate_text(prompt).await
    }
}
