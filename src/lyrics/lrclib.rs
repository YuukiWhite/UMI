use crate::lyrics::parser::{parse_lrc, LyricLine};
use crate::lyrics::sanitizer::sanitize_track;
use crate::storage::LocalDb;
use anyhow::Result;
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum LyricsFetchOutcome {
    Synced(Vec<LyricLine>),
    Plain(Vec<String>),
    NotFound,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LrcLibRecord {
    synced_lyrics: Option<String>,
    plain_lyrics: Option<String>,
    duration: Option<f64>,
}

pub async fn fetch_lyrics(
    db: Option<&LocalDb>,
    raw_artist: &str,
    raw_title: &str,
    duration_sec: Option<u64>,
) -> Result<LyricsFetchOutcome> {
    let clean = sanitize_track(raw_artist, raw_title);
    if clean.title.is_empty() {
        return Ok(LyricsFetchOutcome::NotFound);
    }

    // 1. Check local SQLite cache first (instant + offline capable)
    if let Some(db) = db {
        if let Ok(Some(cached)) = db.get_lyrics(&clean.cache_key) {
            if let Some(synced) = cached.synced_lrc.as_deref() {
                let parsed = parse_lrc(synced);
                if !parsed.is_empty() {
                    return Ok(LyricsFetchOutcome::Synced(parsed));
                }
            }
            if let Some(plain) = cached.plain_lyrics.as_deref() {
                let lines: Vec<String> = plain
                    .lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect();
                if !lines.is_empty() {
                    return Ok(LyricsFetchOutcome::Plain(lines));
                }
            }
        }
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(7))
        .user_agent("UMI-Dynamic-Island/0.2 (https://github.com/umi-notch)")
        .build()?;

    // 2. Exact lookup on LRCLIB /api/get
    let mut best: Option<LrcLibRecord> = None;
    if !clean.artist.is_empty() {
        let mut params = vec![
            ("track_name", clean.title.clone()),
            ("artist_name", clean.artist.clone()),
        ];
        if let Some(dur) = duration_sec.filter(|&d| d > 5) {
            params.push(("duration", dur.to_string()));
        }
        if let Ok(resp) = client
            .get("https://lrclib.net/api/get")
            .query(&params)
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(rec) = resp.json::<LrcLibRecord>().await {
                    if rec.synced_lyrics.is_some() || rec.plain_lyrics.is_some() {
                        best = Some(rec);
                    }
                }
            }
        }
    }

    // 3. Fallback search on LRCLIB /api/search (handles YouTube/YTM variations gracefully)
    if best.as_ref().and_then(|r| r.synced_lyrics.as_ref()).is_none() {
        let q = if clean.artist.is_empty() {
            clean.title.clone()
        } else {
            format!("{} {}", clean.artist, clean.title)
        };
        if let Ok(resp) = client
            .get("https://lrclib.net/api/search")
            .query(&[("q", q)])
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(list) = resp.json::<Vec<LrcLibRecord>>().await {
                    let target_d = duration_sec.map(|d| d as f64);
                    let picked = list
                        .into_iter()
                        .filter(|r| r.synced_lyrics.is_some() || r.plain_lyrics.is_some())
                        .min_by(|a, b| {
                            let score = |r: &LrcLibRecord| -> f64 {
                                let sync_bonus = if r.synced_lyrics.is_some() { 0.0 } else { 1000.0 };
                                let dur_diff = match (target_d, r.duration) {
                                    (Some(t), Some(d)) => (t - d).abs(),
                                    _ => 15.0,
                                };
                                sync_bonus + dur_diff
                            };
                            score(a)
                                .partial_cmp(&score(b))
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                    if picked.is_some() {
                        best = picked;
                    }
                }
            }
        }
    }

    if let Some(rec) = best {
        if let Some(db) = db {
            let _ = db.put_lyrics(
                &clean.cache_key,
                rec.synced_lyrics.as_deref(),
                rec.plain_lyrics.as_deref(),
                duration_sec.map(|d| d as i64),
            );
        }
        if let Some(synced) = rec.synced_lyrics.as_deref() {
            let parsed = parse_lrc(synced);
            if !parsed.is_empty() {
                return Ok(LyricsFetchOutcome::Synced(parsed));
            }
        }
        if let Some(plain) = rec.plain_lyrics.as_deref() {
            let lines: Vec<String> = plain
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();
            if !lines.is_empty() {
                return Ok(LyricsFetchOutcome::Plain(lines));
            }
        }
    }

    Ok(LyricsFetchOutcome::NotFound)
}
