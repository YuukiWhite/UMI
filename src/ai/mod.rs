pub mod quick_math;

use crate::config::AiConfig;
use crate::storage::secrets;
use anyhow::{Context, Result};
use base64::Engine as _;
use serde_json::json;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum AiRequest {
    Ask {
        prompt: String,
        translate_mode: bool,
    },
    SmartOcr,
}

#[derive(Debug, Clone)]
pub struct AiResponse {
    pub prompt_echo: String,
    pub answer: String,
    pub latency_ms: u64,
    pub provider_label: String,
    pub auto_copy: bool,
}

/// Execute Ask UMI AI, Translate, Quick Math, or Smart Screen OCR (KDE Screenshot Portal -> Gemini Vision).
pub async fn handle_request(req: AiRequest, cfg: &AiConfig) -> AiResponse {
    let started = Instant::now();
    match req {
        AiRequest::Ask {
            prompt,
            translate_mode,
        } => {
            let trimmed = prompt.trim();
            if trimmed.is_empty() {
                return AiResponse {
                    prompt_echo: "Ask UMI AI".into(),
                    answer: "Type a question, translation text, or math ending with '='.".into(),
                    latency_ms: 0,
                    provider_label: "Local".into(),
                    auto_copy: false,
                };
            }

            // 1. Instant zero-cost Quick Math & Currency when ending with '='
            if !translate_mode {
                if let Some(local_res) = quick_math::try_eval(trimmed) {
                    return AiResponse {
                        prompt_echo: format!("You: {trimmed}"),
                        answer: format!("= {local_res}"),
                        latency_ms: started.elapsed().as_millis() as u64,
                        provider_label: "Quick Math".into(),
                        auto_copy: true,
                    };
                }
            }

            let effective_prompt = if translate_mode {
                format!(
                    "Translate the following text cleanly and naturally into {}. Return only the translation without extra commentary:\n\n{}",
                    cfg.translate_target_lang, trimmed
                )
            } else {
                format!(
                    "Answer concisely (2-4 short lines or crisp bullet points suitable for a compact desktop Dynamic Island):\n\n{}",
                    trimmed
                )
            };

            let res = match cfg.provider.to_ascii_lowercase().as_str() {
                "claude" => call_claude(&effective_prompt, cfg).await,
                "openai" => call_openai_compat(&effective_prompt, cfg).await,
                "ollama" => call_ollama(&effective_prompt, cfg).await,
                _ => call_gemini_text(&effective_prompt, cfg).await,
            };

            let elapsed = started.elapsed().as_millis() as u64;
            match res {
                Ok((answer, label)) => AiResponse {
                    prompt_echo: format!("You: {trimmed}"),
                    answer,
                    latency_ms: elapsed,
                    provider_label: label,
                    auto_copy: false,
                },
                Err(err) => AiResponse {
                    prompt_echo: format!("You: {trimmed}"),
                    answer: format!("{err}"),
                    latency_ms: elapsed,
                    provider_label: "UMI AI".into(),
                    auto_copy: false,
                },
            }
        }
        AiRequest::SmartOcr => {
            let png = match crate::sys::portals::capture_screenshot_png().await {
                Ok(b) => b,
                Err(err) => {
                    return AiResponse {
                        prompt_echo: "Smart Screen OCR".into(),
                        answer: format!("Capture cancelled or failed: {err}"),
                        latency_ms: started.elapsed().as_millis() as u64,
                        provider_label: "OCR".into(),
                        auto_copy: false,
                    };
                }
            };
            let res = call_gemini_vision_ocr(&png, cfg).await;
            let elapsed = started.elapsed().as_millis() as u64;
            match res {
                Ok(text) => AiResponse {
                    prompt_echo: "Smart Screen OCR · Copied to clipboard".into(),
                    answer: text,
                    latency_ms: elapsed,
                    provider_label: "Gemini Vision".into(),
                    auto_copy: true,
                },
                Err(err) => AiResponse {
                    prompt_echo: "Smart Screen OCR".into(),
                    answer: format!("{err}"),
                    latency_ms: elapsed,
                    provider_label: "Gemini Vision".into(),
                    auto_copy: false,
                },
            }
        }
    }
}

async fn call_gemini_text(prompt: &str, cfg: &AiConfig) -> Result<(String, String)> {
    let key = secrets::read_secret("gemini").await.context(
        "Gemini API key not configured. Run: umi set-key gemini <YOUR_API_KEY>",
    )?;
    let body = json!({
        "contents": [{ "parts": [{ "text": prompt }] }]
    });
    let (v, used_model) = post_gemini_with_fallback(&key, &cfg.gemini_model, &body, 20).await?;
    let text = v["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("(empty response)")
        .trim()
        .to_string();
    Ok((text, format!("Gemini ({})", short_model_label(&used_model))))
}

async fn call_gemini_vision_ocr(png_bytes: &[u8], cfg: &AiConfig) -> Result<String> {
    let key = secrets::read_secret("gemini").await.context(
        "Gemini API key required for Smart OCR. Run: umi set-key gemini <YOUR_API_KEY>",
    )?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(png_bytes);
    let body = json!({
        "contents": [{
            "parts": [
                { "text": "Extract all visible text, code, or equations verbatim from this screenshot. Return only the extracted text without markdown fences or commentary." },
                { "inline_data": { "mime_type": "image/png", "data": b64 } }
            ]
        }]
    });
    let (v, _used_model) = post_gemini_with_fallback(&key, &cfg.gemini_model, &body, 25).await?;
    Ok(v["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("(no text detected)")
        .trim()
        .to_string())
}

/// Calls Gemini `generateContent` with automatic fallback on HTTP 404 / 429 / 5xx,
/// including live discovery from `/v1beta/models` if default aliases are unavailable or overloaded.
async fn post_gemini_with_fallback(
    key: &str,
    preferred_model: &str,
    body: &serde_json::Value,
    timeout_secs: u64,
) -> Result<(serde_json::Value, String)> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()?;

    let raw_candidates = [
        preferred_model.trim_start_matches("models/"),
        "gemini-2.5-flash",
        "gemini-2.5-flash-lite",
        "gemini-2.0-flash",
        "gemini-2.0-flash-lite",
        "gemini-flash-latest",
    ];
    let mut candidates: Vec<String> = Vec::new();
    for m in raw_candidates {
        if !m.is_empty() && !candidates.iter().any(|c| c == m) {
            candidates.push(m.to_string());
        }
    }

    let mut last_err = String::new();
    for model in &candidates {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={key}"
        );
        let resp = match client.post(&url).json(body).send().await {
            Ok(r) => r,
            Err(e) => {
                last_err = format!("Gemini network ({model}): {e}");
                continue;
            }
        };
        let status = resp.status();
        if status.is_success() {
            let v: serde_json::Value = resp.json().await?;
            return Ok((v, model.clone()));
        }
        let err_txt = resp.text().await.unwrap_or_default();
        last_err = format!(
            "Gemini HTTP {status} ({model}): {}",
            err_txt.chars().take(120).collect::<String>()
        );
        // Retry next model automatically on 404 (not found), 429 (quota), or 5xx (e.g. 503 overloaded)
        if status != reqwest::StatusCode::NOT_FOUND
            && status != reqwest::StatusCode::TOO_MANY_REQUESTS
            && !status.is_server_error()
        {
            anyhow::bail!("{last_err}");
        }
    }

    // Live model auto-discovery on this API key if static candidates returned 404/429/5xx
    for discovered in discover_gemini_models(&client, key).await {
        if candidates.contains(&discovered) {
            continue;
        }
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{discovered}:generateContent?key={key}"
        );
        if let Ok(resp) = client.post(&url).json(body).send().await {
            let status = resp.status();
            if status.is_success() {
                if let Ok(v) = resp.json::<serde_json::Value>().await {
                    return Ok((v, discovered));
                }
            } else {
                let err_txt = resp.text().await.unwrap_or_default();
                last_err = format!(
                    "Gemini HTTP {status} ({discovered}): {}",
                    err_txt.chars().take(120).collect::<String>()
                );
            }
        }
    }

    anyhow::bail!("{last_err}")
}

async fn discover_gemini_models(client: &reqwest::Client, key: &str) -> Vec<String> {
    let url = format!("https://generativelanguage.googleapis.com/v1beta/models?key={key}");
    let Ok(resp) = client.get(url).send().await else {
        return Vec::new();
    };
    if !resp.status().is_success() {
        return Vec::new();
    }
    let Ok(v) = resp.json::<serde_json::Value>().await else {
        return Vec::new();
    };
    let Some(models) = v["models"].as_array() else {
        return Vec::new();
    };
    let mut flash = Vec::new();
    let mut others = Vec::new();
    for m in models {
        let Some(name) = m["name"].as_str() else { continue; };
        let supports_gen = m["supportedGenerationMethods"]
            .as_array()
            .map(|arr| arr.iter().any(|x| x.as_str() == Some("generateContent")))
            .unwrap_or(false);
        if !supports_gen {
            continue;
        }
        let short = name.trim_start_matches("models/");
        if short.contains("tts") || short.contains("image") || short.contains("embedding") {
            continue;
        }
        if short.contains("flash") {
            flash.push(short.to_string());
        } else if short.starts_with("gemini-") {
            others.push(short.to_string());
        }
    }
    flash.extend(others);
    flash.truncate(6);
    flash
}

fn short_model_label(model: &str) -> &str {
    model.strip_prefix("gemini-").unwrap_or(model)
}

async fn call_claude(prompt: &str, cfg: &AiConfig) -> Result<(String, String)> {
    let key = secrets::read_secret("claude").await.context(
        "Claude API key not configured. Run: umi set-key claude <YOUR_API_KEY>",
    )?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()?;
    let body = json!({
        "model": cfg.claude_model,
        "max_tokens": 300,
        "messages": [{ "role": "user", "content": prompt }]
    });
    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await?;
    if !resp.status().is_success() {
        anyhow::bail!("Claude HTTP {}", resp.status());
    }
    let v: serde_json::Value = resp.json().await?;
    let text = v["content"][0]["text"]
        .as_str()
        .unwrap_or("(empty response)")
        .trim()
        .to_string();
    Ok((text, "Claude".into()))
}

async fn call_openai_compat(prompt: &str, cfg: &AiConfig) -> Result<(String, String)> {
    let key = secrets::read_secret("openai").await.context(
        "OpenAI API key not configured. Run: umi set-key openai <YOUR_API_KEY>",
    )?;
    let url = format!("{}/chat/completions", cfg.openai_base_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()?;
    let body = json!({
        "model": cfg.openai_model,
        "messages": [{ "role": "user", "content": prompt }]
    });
    let resp = client.post(url).bearer_auth(key).json(&body).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("OpenAI HTTP {}", resp.status());
    }
    let v: serde_json::Value = resp.json().await?;
    let text = v["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("(empty response)")
        .trim()
        .to_string();
    Ok((text, "OpenAI".into()))
}

async fn call_ollama(prompt: &str, cfg: &AiConfig) -> Result<(String, String)> {
    let url = format!("{}/api/generate", cfg.ollama_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    let body = json!({
        "model": cfg.ollama_model,
        "prompt": prompt,
        "stream": false
    });
    let resp = client.post(url).json(&body).send().await.context(
        "Ollama unreachable at http://127.0.0.1:11434 — start Ollama first",
    )?;
    if !resp.status().is_success() {
        anyhow::bail!("Ollama HTTP {}", resp.status());
    }
    let v: serde_json::Value = resp.json().await?;
    let text = v["response"]
        .as_str()
        .unwrap_or("(empty response)")
        .trim()
        .to_string();
    Ok((text, "Ollama".into()))
}
