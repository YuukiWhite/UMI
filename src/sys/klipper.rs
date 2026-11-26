use crate::state::{AppEvent, ClipboardEntry};
use std::collections::HashMap;
use std::time::Duration;
use zbus::{Connection, Proxy};

/// Sync recent clipboard history from KDE Klipper (`org.kde.klipper`) and execute copy commands.
pub async fn run_klipper_watcher(
    tx: async_channel::Sender<AppEvent>,
    copy_rx: async_channel::Receiver<String>,
) {
    let conn = match Connection::session().await {
        Ok(c) => c,
        Err(err) => {
            tracing::warn!("Klipper watcher skipped: {err}");
            return;
        }
    };

    let proxy = match Proxy::new(&conn, "org.kde.klipper", "/klipper", "org.kde.klipper.klipper").await
    {
        Ok(p) => p,
        Err(err) => {
            tracing::warn!("Klipper D-Bus proxy unavailable: {err}");
            return;
        }
    };

    let mut seen_timestamps: HashMap<String, i64> = HashMap::new();
    let mut last_snapshot: Vec<String> = Vec::new();
    let mut interval = tokio::time::interval(Duration::from_secs(2));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let mut entries = Vec::new();
                let mut texts = Vec::new();
                let now = chrono::Utc::now().timestamp();

                for idx in 0..8i32 {
                    let Ok(item) = proxy.call::<_, _, String>("getClipboardHistoryItem", &(idx,)).await else {
                        break;
                    };
                    let trimmed = item.trim();
                    if trimmed.is_empty() || is_sensitive_secret(trimmed) {
                        continue;
                    }
                    let text = trimmed.chars().take(160).collect::<String>();
                    if texts.contains(&text) {
                        continue;
                    }
                    let ts = *seen_timestamps.entry(text.clone()).or_insert(now);
                    texts.push(text.clone());
                    entries.push(ClipboardEntry {
                        text,
                        captured_at: ts,
                    });
                    if entries.len() >= 6 {
                        break;
                    }
                }

                if texts != last_snapshot {
                    last_snapshot = texts;
                    let _ = tx.send(AppEvent::ClipboardUpdated(entries)).await;
                }
            }
            Ok(to_copy) = copy_rx.recv() => {
                if let Err(err) = proxy.call::<_, _, ()>("setClipboardContents", &(to_copy,)).await {
                    tracing::warn!("Failed to set Klipper clipboard contents: {err}");
                }
            }
        }
    }
}

/// Basic heuristic guard to skip password-manager tokens & private keys in clipboard history.
fn is_sensitive_secret(s: &str) -> bool {
    let low = s.to_ascii_lowercase();
    low.contains("-----begin ")
        || low.contains("x-kde-passwordmanagerhint")
        || s.starts_with("AIzaSy")
        || s.starts_with("sk-ant-")
        || s.starts_with("sk-proj-")
        || s.starts_with("ghp_")
}
