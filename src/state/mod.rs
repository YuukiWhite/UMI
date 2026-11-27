pub mod machine;
pub mod spring;

use crate::ai::AiResponse;
use crate::config::{NotchPageId, UmiConfig};
use crate::lyrics::LyricLine;
use crate::quota::QuotaSnapshot;
use crate::sys::telemetry::WeatherSnapshot;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone, Default)]
pub struct MediaSnapshot {
    pub bus_name: Option<String>,
    pub title: String,
    pub artist: String,
    pub playing: bool,
    pub duration_us: i64,
    pub position_us: i64,
    pub position_synced_at: Option<Instant>,
    pub art_path: Option<PathBuf>,
}

impl MediaSnapshot {
    pub fn has_track(&self) -> bool {
        !self.title.is_empty()
    }

    /// Interpolate real-time playback position in microseconds between MPRIS2 updates.
    pub fn live_position_us(&self) -> i64 {
        let base = self.position_us.max(0);
        let pos = if self.playing {
            if let Some(synced) = self.position_synced_at {
                base + synced.elapsed().as_micros() as i64
            } else {
                base
            }
        } else {
            base
        };
        if self.duration_us > 0 {
            pos.clamp(0, self.duration_us)
        } else {
            pos
        }
    }
}

#[derive(Debug, Clone)]
pub enum LyricsData {
    Idle,
    Loading,
    Synced(Vec<LyricLine>),
    Plain(Vec<String>),
    NotFound,
}

#[derive(Debug, Clone)]
pub struct ClipboardEntry {
    pub text: String,
    pub captured_at: i64,
}

#[derive(Debug, Clone)]
pub enum MediaCommand {
    PlayPause,
    Next,
    Previous,
    SeekRatio(f64),
    SeekMs(u64),
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    ToggleNotch,
    OpenAskAi,
    TriggerOcr,
    OpenSettings,
    JumpToPage(NotchPageId),
    ToggleStartAtLogin,
    ToggleMascotEyes,
    ToggleMinimalMode,
    Quit,
    // Phase 2 live KDE Wayland events
    BatteryUpdated {
        pct: u8,
        #[allow(dead_code)]
        charging: bool,
        just_plugged_in: bool,
    },
    BluetoothConnected {
        device: String,
        battery_pct: u8,
    },
    NotificationReceived {
        app: String,
        sender: String,
        body: String,
    },
    MediaUpdated(MediaSnapshot),
    LyricsUpdated {
        track_key: String,
        data: LyricsData,
    },
    ClipboardUpdated(Vec<ClipboardEntry>),
    PrayerLocationDetected {
        city: String,
        latitude: f64,
        longitude: f64,
    },
    // Phase 4 AI & Quota events
    AiResponseReady(AiResponse),
    QuotasUpdated(Vec<QuotaSnapshot>),
    QuotaLowAlert {
        provider: String,
        remaining_pct: u8,
        resets_in: String,
    },
    // Phase 5 live Settings synchronization
    ConfigChanged(UmiConfig),
    // v1.4 live weather widget snapshot
    WeatherUpdated(WeatherSnapshot),
}
