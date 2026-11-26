use crate::config::AiConfig;
use crate::state::AppEvent;
use crate::storage::LocalDb;
use directories::{BaseDirs, ProjectDirs};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuotaStatus {
    Ok,
    Low,
    Exhausted,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaSnapshot {
    pub provider_id: String,
    pub display_name: String,
    pub plan_label: String,
    pub remaining_pct: u8,
    pub resets_in_sec: Option<u64>,
    pub status: QuotaStatus,
    pub accent_hex: String,
    pub updated_at: i64,
}

impl QuotaSnapshot {
    pub fn reset_label(&self) -> String {
        if self.status == QuotaStatus::Unavailable {
            return if self.provider_id.starts_with("codex") {
                "Run codex /status".to_string()
            } else if self.provider_id == "antigravity" {
                "Add script / agy".to_string()
            } else {
                "Add quota script".to_string()
            };
        }
        let Some(secs) = self.resets_in_sec else {
            return "Window reset".to_string();
        };
        if secs >= 86_400 {
            let d = secs / 86_400;
            let h = (secs % 86_400) / 3600;
            format!("Resets in {d}d {h}h")
        } else if secs >= 3600 {
            let h = secs / 3600;
            let m = (secs % 3600) / 60;
            format!("Resets in {h}h {m}m")
        } else {
            let m = (secs / 60).max(1);
            format!("Resets in {m}m")
        }
    }
}

#[derive(Debug, Deserialize)]
struct ScriptQuotaJson {
    provider: String,
    display_name: Option<String>,
    plan: Option<String>,
    remaining_pct: u8,
    resets_in_sec: Option<u64>,
    status: Option<String>,
}

pub fn quota_scripts_dir() -> Option<PathBuf> {
    let dirs = ProjectDirs::from("org", "umi", "umi")?;
    Some(dirs.config_dir().join("quota-scripts"))
}

/// Parse standardized ScriptProvider JSON stdout (`{provider, display_name, plan, remaining_pct, resets_in_sec, status}`).
pub fn parse_script_json(raw: &str, low_threshold: u8) -> Option<QuotaSnapshot> {
    let parsed: ScriptQuotaJson = serde_json::from_str(raw.trim()).ok()?;
    let pct = parsed.remaining_pct.min(100);
    let pid = parsed.provider.trim().to_ascii_lowercase();
    let display_name = parsed
        .display_name
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| capitalize(&pid));
    let plan_label = parsed
        .plan
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "Live · Script".to_string());
    let status = match parsed.status.as_deref() {
        Some("exhausted") => QuotaStatus::Exhausted,
        Some("low") => QuotaStatus::Low,
        Some("unavailable") => QuotaStatus::Unavailable,
        _ if pct == 0 => QuotaStatus::Exhausted,
        _ if pct < low_threshold => QuotaStatus::Low,
        _ => QuotaStatus::Ok,
    };
    let accent_hex = default_accent_for(&pid).to_string();
    Some(QuotaSnapshot {
        provider_id: pid,
        display_name,
        plan_label,
        remaining_pct: pct,
        resets_in_sec: parsed.resets_in_sec,
        status,
        accent_hex,
        updated_at: chrono::Utc::now().timestamp(),
    })
}

fn default_accent_for(id: &str) -> &'static str {
    if id.starts_with("antigravity") {
        "#38BDF8"
    } else if id.starts_with("codex") {
        "#10B981"
    } else if id.starts_with("claude") {
        "#F59E0B"
    } else {
        "#A855F7"
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

/// Collect real, honest quota data for user-enabled providers + custom ScriptProviders.
/// Never invents hardcoded percentages or shows disabled providers.
pub async fn collect_all_quotas(enabled_providers: &[String], low_threshold: u8) -> Vec<QuotaSnapshot> {
    let now = chrono::Utc::now().timestamp();
    let home = BaseDirs::new().map(|b| b.home_dir().to_path_buf());
    let mut slots: Vec<QuotaSnapshot> = Vec::new();

    let want = |name: &str| {
        enabled_providers
            .iter()
            .any(|p| p.eq_ignore_ascii_case(name))
    };

    // 1. Custom ScriptProviders in ~/.config/umi/quota-scripts/*.sh (override matching builtin or append)
    let mut script_snaps: Vec<QuotaSnapshot> = Vec::new();
    if let Some(dir) = quota_scripts_dir() {
        let _ = fs::create_dir_all(&dir);
        if let Ok(entries) = fs::read_dir(&dir) {
            let mut paths: Vec<PathBuf> = entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("sh"))
                .collect();
            paths.sort();
            for path in paths {
                if let Some(snap) = run_quota_script(&path, low_threshold).await {
                    script_snaps.push(snap);
                }
            }
        }
    }

    // 2. Antigravity (only if enabled; checks user script -> umi_quota.json -> honest Unavailable)
    if want("antigravity") {
        if let Some(snap) = take_script_snap(&mut script_snaps, "antigravity") {
            slots.push(snap);
        } else if let Some(snap) = home
            .as_ref()
            .and_then(|h| read_dropin_json(&h.join(".gemini/antigravity/umi_quota.json"), low_threshold))
        {
            slots.push(snap);
        } else {
            slots.push(QuotaSnapshot {
                provider_id: "antigravity".to_string(),
                display_name: "Antigravity".to_string(),
                plan_label: "No local quota file".to_string(),
                remaining_pct: 0,
                resets_in_sec: None,
                status: QuotaStatus::Unavailable,
                accent_hex: default_accent_for("antigravity").to_string(),
                updated_at: now,
            });
        }
    }

    // 3. Codex (real-time reader from ~/.codex/sessions/**/*.jsonl rate_limits events)
    if want("codex") {
        if let Some(snap) = take_script_snap(&mut script_snaps, "codex") {
            slots.push(snap);
        } else if let Some(ref h) = home {
            let split_two_cards = enabled_providers.len() <= 2;
            let codex_cards = read_codex_session_quotas(
                &h.join(".codex/sessions"),
                now,
                low_threshold,
                split_two_cards,
            );
            if !codex_cards.is_empty() {
                slots.extend(codex_cards);
            } else if let Some(snap) = read_dropin_json(&h.join(".codex/umi_quota.json"), low_threshold) {
                slots.push(snap);
            } else {
                slots.push(QuotaSnapshot {
                    provider_id: "codex".to_string(),
                    display_name: "Codex".to_string(),
                    plan_label: "No recent session".to_string(),
                    remaining_pct: 0,
                    resets_in_sec: None,
                    status: QuotaStatus::Unavailable,
                    accent_hex: default_accent_for("codex").to_string(),
                    updated_at: now,
                });
            }
        }
    }

    // 4. Claude (only if explicitly enabled by user; never fabricated!)
    if want("claude") {
        if let Some(snap) = take_script_snap(&mut script_snaps, "claude") {
            slots.push(snap);
        } else if let Some(snap) = home
            .as_ref()
            .and_then(|h| read_dropin_json(&h.join(".claude/umi_quota.json"), low_threshold))
        {
            slots.push(snap);
        } else {
            slots.push(QuotaSnapshot {
                provider_id: "claude".to_string(),
                display_name: "Claude".to_string(),
                plan_label: "Not configured".to_string(),
                remaining_pct: 0,
                resets_in_sec: None,
                status: QuotaStatus::Unavailable,
                accent_hex: default_accent_for("claude").to_string(),
                updated_at: now,
            });
        }
    }

    // 5. Append any other custom ScriptProviders (e.g. Cursor, Copilot, etc.)
    for extra in script_snaps {
        if !slots.iter().any(|s| s.provider_id == extra.provider_id) {
            slots.push(extra);
        }
    }

    slots
}

fn take_script_snap(list: &mut Vec<QuotaSnapshot>, id: &str) -> Option<QuotaSnapshot> {
    let idx = list.iter().position(|s| s.provider_id == id)?;
    Some(list.remove(idx))
}

fn read_dropin_json(path: &Path, low_threshold: u8) -> Option<QuotaSnapshot> {
    let raw = fs::read_to_string(path).ok()?;
    parse_script_json(&raw, low_threshold)
}

#[derive(Debug, Clone)]
struct CodexWindow {
    used_percent: f64,
    window_minutes: u64,
    resets_at: Option<i64>,
}

#[derive(Debug, Clone)]
struct CodexRateLimits {
    plan_type: Option<String>,
    primary: Option<CodexWindow>,
    secondary: Option<CodexWindow>,
}

/// Reads the latest real `rate_limits` payload from Codex CLI local session logs (`~/.codex/sessions/**/*.jsonl`).
fn read_codex_session_quotas(
    sessions_dir: &Path,
    now_unix: i64,
    low_threshold: u8,
    split_two_cards: bool,
) -> Vec<QuotaSnapshot> {
    let files = newest_jsonl_files(sessions_dir, 10);
    for path in files {
        if let Some(rl) = extract_latest_codex_rate_limits(&path) {
            return build_codex_snapshots(&rl, now_unix, low_threshold, split_two_cards);
        }
    }
    Vec::new()
}

fn newest_jsonl_files(root: &Path, limit: usize) -> Vec<PathBuf> {
    if !root.is_dir() {
        return Vec::new();
    }
    let mut found: Vec<(SystemTime, PathBuf)> = Vec::new();
    collect_jsonl_recursive(root, 0, &mut found);
    found.sort_by(|a, b| b.0.cmp(&a.0));
    found.into_iter().take(limit).map(|(_, p)| p).collect()
}

fn collect_jsonl_recursive(dir: &Path, depth: usize, out: &mut Vec<(SystemTime, PathBuf)>) {
    if depth > 4 {
        return;
    }
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    for entry in rd.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_jsonl_recursive(&path, depth + 1, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
            let mtime = entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            out.push((mtime, path));
        }
    }
}

/// Efficiently read the tail (up to 512 KB) of a Codex `.jsonl` session file and return the newest `rate_limits`.
fn extract_latest_codex_rate_limits(path: &Path) -> Option<CodexRateLimits> {
    let mut file = fs::File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    let max_tail: u64 = 512 * 1024;
    if len > max_tail {
        let _ = file.seek(SeekFrom::Start(len - max_tail));
    }
    let mut buf = String::new();
    file.read_to_string(&mut buf).ok()?;

    for line in buf.lines().rev() {
        if !line.contains("rate_limits") && !line.contains("rateLimits") {
            continue;
        }
        if let Some(rl) = parse_codex_rate_limits_line(line) {
            return Some(rl);
        }
    }
    None
}

fn parse_codex_rate_limits_line(line: &str) -> Option<CodexRateLimits> {
    let v: Value = serde_json::from_str(line).ok()?;
    let rl = find_rate_limits_obj(&v)?;
    let plan_type = rl
        .get("plan_type")
        .or_else(|| rl.get("planType"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let primary = rl.get("primary").and_then(parse_codex_window);
    let secondary = rl.get("secondary").and_then(parse_codex_window);
    if primary.is_none() && secondary.is_none() {
        return None;
    }
    Some(CodexRateLimits {
        plan_type,
        primary,
        secondary,
    })
}

fn find_rate_limits_obj(v: &Value) -> Option<&Value> {
    for key in ["rate_limits", "rateLimits"] {
        if let Some(obj) = v.get(key) {
            if obj.is_object() {
                return Some(obj);
            }
        }
    }
    if let Some(payload) = v.get("payload") {
        if let Some(found) = find_rate_limits_obj(payload) {
            return Some(found);
        }
    }
    if let Some(info) = v.get("info") {
        if let Some(found) = find_rate_limits_obj(info) {
            return Some(found);
        }
    }
    None
}

fn parse_codex_window(v: &Value) -> Option<CodexWindow> {
    if !v.is_object() {
        return None;
    }
    let used_percent = v
        .get("used_percent")
        .or_else(|| v.get("usedPercent"))
        .and_then(Value::as_f64)?;
    let window_minutes = v
        .get("window_minutes")
        .or_else(|| v.get("windowMinutes"))
        .and_then(Value::as_u64)
        .unwrap_or(300);
    let resets_at = v
        .get("resets_at")
        .or_else(|| v.get("resetsAt"))
        .and_then(|x| x.as_i64().or_else(|| x.as_f64().map(|f| f as i64)));
    Some(CodexWindow {
        used_percent,
        window_minutes,
        resets_at,
    })
}

fn eval_window(w: &CodexWindow, now_unix: i64) -> (u8, Option<u64>) {
    if let Some(reset_ts) = w.resets_at {
        if now_unix >= reset_ts {
            return (100, None);
        }
        let rem_pct = (100.0 - w.used_percent).round().clamp(0.0, 100.0) as u8;
        return (rem_pct, Some((reset_ts - now_unix) as u64));
    }
    let rem_pct = (100.0 - w.used_percent).round().clamp(0.0, 100.0) as u8;
    (rem_pct, None)
}

fn window_short_name(minutes: u64) -> String {
    if minutes >= 1440 * 6 {
        "Weekly".to_string()
    } else if minutes >= 60 {
        format!("{}h window", minutes / 60)
    } else {
        format!("{minutes}m")
    }
}

fn status_for_pct(pct: u8, low_threshold: u8) -> QuotaStatus {
    if pct == 0 {
        QuotaStatus::Exhausted
    } else if pct < low_threshold {
        QuotaStatus::Low
    } else {
        QuotaStatus::Ok
    }
}

fn build_codex_snapshots(
    rl: &CodexRateLimits,
    now_unix: i64,
    low_threshold: u8,
    split_two_cards: bool,
) -> Vec<QuotaSnapshot> {
    let plan = rl
        .plan_type
        .as_deref()
        .map(capitalize)
        .unwrap_or_else(|| "Codex".to_string());

    if split_two_cards {
        let mut out = Vec::new();
        if let Some(ref p) = rl.primary {
            let (rem, resets) = eval_window(p, now_unix);
            out.push(QuotaSnapshot {
                provider_id: "codex_5h".to_string(),
                display_name: "Codex 5h".to_string(),
                plan_label: format!("{plan} · {}", window_short_name(p.window_minutes)),
                remaining_pct: rem,
                resets_in_sec: resets,
                status: status_for_pct(rem, low_threshold),
                accent_hex: default_accent_for("codex").to_string(),
                updated_at: now_unix,
            });
        }
        if let Some(ref s) = rl.secondary {
            let (rem, resets) = eval_window(s, now_unix);
            out.push(QuotaSnapshot {
                provider_id: "codex_weekly".to_string(),
                display_name: "Codex Weekly".to_string(),
                plan_label: format!("{plan} · {}", window_short_name(s.window_minutes)),
                remaining_pct: rem,
                resets_in_sec: resets,
                status: status_for_pct(rem, low_threshold),
                accent_hex: default_accent_for("codex").to_string(),
                updated_at: now_unix,
            });
        }
        if !out.is_empty() {
            return out;
        }
    }

    // Combined single card (shows weekly remaining + 5h in subtitle, or primary if weekly missing)
    match (&rl.primary, &rl.secondary) {
        (Some(p), Some(s)) => {
            let (p_rem, p_res) = eval_window(p, now_unix);
            let (s_rem, s_res) = eval_window(s, now_unix);
            let (rem, resets, label) = if s_rem <= p_rem {
                (s_rem, s_res, format!("{plan} · Wk · 5h {p_rem}%"))
            } else {
                (p_rem, p_res, format!("{plan} · 5h · Wk {s_rem}%"))
            };
            vec![QuotaSnapshot {
                provider_id: "codex".to_string(),
                display_name: "Codex".to_string(),
                plan_label: label,
                remaining_pct: rem,
                resets_in_sec: resets,
                status: status_for_pct(rem, low_threshold),
                accent_hex: default_accent_for("codex").to_string(),
                updated_at: now_unix,
            }]
        }
        (Some(w), None) | (None, Some(w)) => {
            let (rem, resets) = eval_window(w, now_unix);
            vec![QuotaSnapshot {
                provider_id: "codex".to_string(),
                display_name: "Codex".to_string(),
                plan_label: format!("{plan} · {}", window_short_name(w.window_minutes)),
                remaining_pct: rem,
                resets_in_sec: resets,
                status: status_for_pct(rem, low_threshold),
                accent_hex: default_accent_for("codex").to_string(),
                updated_at: now_unix,
            }]
        }
        (None, None) => Vec::new(),
    }
}

async fn run_quota_script(path: &PathBuf, low_threshold: u8) -> Option<QuotaSnapshot> {
    let fut = tokio::process::Command::new(path).output();
    let out = tokio::time::timeout(Duration::from_secs(3), fut).await.ok()?.ok()?;
    if !out.status.success() || out.stdout.len() > 64 * 1024 {
        return None;
    }
    let txt = String::from_utf8(out.stdout).ok()?;
    parse_script_json(&txt, low_threshold)
}

/// Periodic AI Limits quota watcher (15s lightweight local poll) + `< threshold` Peek alert emitter.
pub async fn run_quota_watcher(
    tx: async_channel::Sender<AppEvent>,
    refresh_rx: async_channel::Receiver<()>,
    db: Option<LocalDb>,
    ai_cfg: Arc<RwLock<AiConfig>>,
) {
    let mut alerted: HashSet<String> = HashSet::new();
    let mut interval = tokio::time::interval(Duration::from_secs(15));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        let (providers, low_threshold) = ai_cfg
            .read()
            .map(|c| (c.quota_providers.clone(), c.low_quota_threshold_pct))
            .unwrap_or_else(|_| (vec!["codex".into()], 20));

        let snaps = collect_all_quotas(&providers, low_threshold).await;
        if let Some(ref db) = db {
            let _ = db.save_quota_snapshots(&snaps);
        }

        for s in &snaps {
            if matches!(s.status, QuotaStatus::Low | QuotaStatus::Exhausted)
                && s.remaining_pct < low_threshold
                && !alerted.contains(&s.provider_id)
            {
                alerted.insert(s.provider_id.clone());
                let _ = tx
                    .send(AppEvent::QuotaLowAlert {
                        provider: s.display_name.clone(),
                        remaining_pct: s.remaining_pct,
                        resets_in: s.reset_label().replace("Resets in ", ""),
                    })
                    .await;
            } else if s.remaining_pct >= low_threshold || s.status == QuotaStatus::Unavailable {
                alerted.remove(&s.provider_id);
            }
        }

        let _ = tx.send(AppEvent::QuotasUpdated(snaps)).await;

        tokio::select! {
            _ = interval.tick() => {}
            Ok(()) = refresh_rx.recv() => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_script_provider_json_and_low_threshold() {
        let json = r#"{
            "provider": "antigravity",
            "display_name": "Antigravity",
            "plan": "Pro · 5h window",
            "remaining_pct": 18,
            "resets_in_sec": 2880,
            "status": "low"
        }"#;
        let snap = parse_script_json(json, 20).expect("valid quota json");
        assert_eq!(snap.provider_id, "antigravity");
        assert_eq!(snap.remaining_pct, 18);
        assert_eq!(snap.status, QuotaStatus::Low);
        assert_eq!(snap.reset_label(), "Resets in 48m");
    }

    #[test]
    fn parses_real_codex_jsonl_rate_limits_and_handles_reset() {
        let now = 1_800_000_000_i64;
        let line = r#"{"type":"event_msg","payload":{"type":"token_count","rate_limits":{"limit_id":"codex","plan_type":"plus","primary":{"used_percent":25.0,"window_minutes":300,"resets_at":1800007200},"secondary":{"used_percent":12.0,"window_minutes":10080,"resets_at":1800259200}}}}"#;
        let rl = parse_codex_rate_limits_line(line).expect("codex rate_limits");
        let cards = build_codex_snapshots(&rl, now, 20, true);
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].display_name, "Codex 5h");
        assert_eq!(cards[0].remaining_pct, 75);
        assert_eq!(cards[0].reset_label(), "Resets in 2h 0m");
        assert_eq!(cards[1].display_name, "Codex Weekly");
        assert_eq!(cards[1].remaining_pct, 88);
        assert_eq!(cards[1].reset_label(), "Resets in 3d 0h");

        // Expired reset timestamp automatically reports 100% remaining
        let expired = build_codex_snapshots(&rl, 1_800_500_000, 20, true);
        assert_eq!(expired[0].remaining_pct, 100);
        assert_eq!(expired[1].remaining_pct, 100);
    }
}
