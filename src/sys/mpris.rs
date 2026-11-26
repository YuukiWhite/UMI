use crate::lyrics::{fetch_lyrics, sanitize_track, LyricsFetchOutcome};
use crate::state::{AppEvent, LyricsData, MediaCommand, MediaSnapshot};
use crate::storage::{art_cache_dir, LocalDb};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use zbus::fdo::DBusProxy;
use zbus::zvariant::OwnedValue;
use zbus::{Connection, Proxy};

/// Monitor MPRIS2 players (Spotify, YouTube / YouTube Music in browsers, VLC, etc.),
/// forward playback controls, download album art, and auto-fetch LRCLIB synced lyrics.
pub async fn run_mpris_watcher(
    tx: async_channel::Sender<AppEvent>,
    cmd_rx: async_channel::Receiver<MediaCommand>,
    db: Option<LocalDb>,
) {
    let conn = match Connection::session().await {
        Ok(c) => c,
        Err(err) => {
            tracing::warn!("MPRIS2 watcher skipped: {err}");
            return;
        }
    };

    let mut active_bus: Option<String> = None;
    let mut last_track_key = String::new();
    let mut last_art_url = String::new();
    let mut cached_art_path: Option<PathBuf> = None;
    let mut last_duration_us: i64 = 0;

    let mut interval = tokio::time::interval(Duration::from_millis(900));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let snap = poll_best_player(&conn).await;
                if let Some(mut media) = snap {
                    active_bus = media.bus_name.clone();
                    last_duration_us = media.duration_us;

                    let clean = sanitize_track(&media.artist, &media.title);
                    let track_key = clean.cache_key.clone();

                    // Resolve album art when artUrl changes
                    let art_url_str = media
                        .art_path
                        .as_ref()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default();
                    if art_url_str != last_art_url {
                        last_art_url = art_url_str.clone();
                        cached_art_path = resolve_art_path(&art_url_str).await;
                    }
                    media.art_path = cached_art_path.clone();

                    // Trigger async LRCLIB lyrics lookup when song changes
                    if !track_key.is_empty() && track_key != last_track_key {
                        last_track_key = track_key.clone();
                        let tx_l = tx.clone();
                        let db_l = db.clone();
                        let raw_artist = media.artist.clone();
                        let raw_title = media.title.clone();
                        let dur_sec = if media.duration_us > 0 {
                            Some((media.duration_us / 1_000_000) as u64)
                        } else {
                            None
                        };
                        let _ = tx_l
                            .send(AppEvent::LyricsUpdated {
                                track_key: track_key.clone(),
                                data: LyricsData::Loading,
                            })
                            .await;
                        tokio::spawn(async move {
                            let data = match fetch_lyrics(db_l.as_ref(), &raw_artist, &raw_title, dur_sec).await {
                                Ok(LyricsFetchOutcome::Synced(lines)) => LyricsData::Synced(lines),
                                Ok(LyricsFetchOutcome::Plain(lines)) => LyricsData::Plain(lines),
                                _ => LyricsData::NotFound,
                            };
                            let _ = tx_l
                                .send(AppEvent::LyricsUpdated { track_key, data })
                                .await;
                        });
                    }

                    let _ = tx.send(AppEvent::MediaUpdated(media)).await;
                } else if active_bus.is_some() {
                    active_bus = None;
                    last_track_key.clear();
                    last_art_url.clear();
                    cached_art_path = None;
                    let _ = tx.send(AppEvent::MediaUpdated(MediaSnapshot::default())).await;
                }
            }
            Ok(cmd) = cmd_rx.recv() => {
                let Some(ref bus) = active_bus else { continue; };
                if let Err(err) = execute_media_command(&conn, bus, cmd, last_duration_us).await {
                    tracing::warn!("MPRIS2 command failed on {bus}: {err}");
                }
            }
        }
    }
}

async fn poll_best_player(conn: &Connection) -> Option<MediaSnapshot> {
    let dbus = DBusProxy::new(conn).await.ok()?;
    let names = dbus.list_names().await.ok()?;

    let mut best: Option<MediaSnapshot> = None;

    for owned_name in names {
        let name = owned_name.as_str();
        if !name.starts_with("org.mpris.MediaPlayer2.") {
            continue;
        }
        let Ok(proxy) = Proxy::new(
            conn,
            name,
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player",
        )
        .await
        else {
            continue;
        };

        let status: String = proxy.get_property("PlaybackStatus").await.unwrap_or_default();
        let playing = status == "Playing";
        let meta: HashMap<String, OwnedValue> = proxy.get_property("Metadata").await.unwrap_or_default();

        let title = meta
            .get("xesam:title")
            .and_then(|v| String::try_from(v.clone()).ok())
            .unwrap_or_default();
        if title.trim().is_empty() {
            continue;
        }

        let artist = meta
            .get("xesam:artist")
            .and_then(|v| Vec::<String>::try_from(v.clone()).ok())
            .map(|v| v.join(", "))
            .or_else(|| {
                meta.get("xesam:artist")
                    .and_then(|v| String::try_from(v.clone()).ok())
            })
            .unwrap_or_default();

        let duration_us = meta
            .get("mpris:length")
            .and_then(|v| {
                i64::try_from(v.clone())
                    .ok()
                    .or_else(|| u64::try_from(v.clone()).ok().map(|u| u as i64))
            })
            .unwrap_or(0);

        let art_url = meta
            .get("mpris:artUrl")
            .and_then(|v| String::try_from(v.clone()).ok())
            .filter(|s| !s.is_empty())
            .map(PathBuf::from);

        let position_us: i64 = proxy.get_property("Position").await.unwrap_or(0);

        let candidate = MediaSnapshot {
            bus_name: Some(name.to_string()),
            title,
            artist,
            playing,
            duration_us,
            position_us,
            position_synced_at: Some(Instant::now()),
            art_path: art_url,
        };

        if playing {
            return Some(candidate);
        }
        if best.is_none() {
            best = Some(candidate);
        }
    }

    best
}

async fn execute_media_command(
    conn: &Connection,
    bus_name: &str,
    cmd: MediaCommand,
    duration_us: i64,
) -> zbus::Result<()> {
    let proxy = Proxy::new(
        conn,
        bus_name,
        "/org/mpris/MediaPlayer2",
        "org.mpris.MediaPlayer2.Player",
    )
    .await?;

    match cmd {
        MediaCommand::PlayPause => proxy.call::<_, _, ()>("PlayPause", &()).await?,
        MediaCommand::Next => proxy.call::<_, _, ()>("Next", &()).await?,
        MediaCommand::Previous => proxy.call::<_, _, ()>("Previous", &()).await?,
        MediaCommand::SeekRatio(ratio) => {
            if duration_us > 0 {
                let target_us = ((ratio.clamp(0.0, 1.0)) * duration_us as f64) as i64;
                seek_to_us(&proxy, target_us).await?;
            }
        }
        MediaCommand::SeekMs(ms) => {
            seek_to_us(&proxy, (ms * 1000) as i64).await?;
        }
    }
    Ok(())
}

async fn seek_to_us(proxy: &Proxy<'_>, target_us: i64) -> zbus::Result<()> {
    let meta: HashMap<String, OwnedValue> = proxy.get_property("Metadata").await.unwrap_or_default();
    if let Some(track_id) = meta
        .get("mpris:trackid")
        .and_then(|v| zbus::zvariant::OwnedObjectPath::try_from(v.clone()).ok())
    {
        proxy
            .call::<_, _, ()>("SetPosition", &(track_id, target_us))
            .await?;
    } else {
        let cur: i64 = proxy.get_property("Position").await.unwrap_or(0);
        proxy.call::<_, _, ()>("Seek", &(target_us - cur)).await?;
    }
    Ok(())
}

async fn resolve_art_path(art_url: &str) -> Option<PathBuf> {
    if art_url.is_empty() {
        return None;
    }
    if let Some(local) = art_url.strip_prefix("file://") {
        let decoded = local.replace("%20", " ");
        let p = PathBuf::from(decoded);
        return p.exists().then_some(p);
    }
    if art_url.starts_with("http://") || art_url.starts_with("https://") {
        let dir = art_cache_dir().ok()?;
        let hash = simple_hash(art_url);
        let dest = dir.join(format!("{hash:016x}.img"));
        if dest.exists() {
            return Some(dest);
        }
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .ok()?;
        let bytes = client.get(art_url).send().await.ok()?.bytes().await.ok()?;
        if !bytes.is_empty() && bytes.len() < 8 * 1024 * 1024 {
            std::fs::write(&dest, &bytes).ok()?;
            return Some(dest);
        }
    }
    None
}

fn simple_hash(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}
