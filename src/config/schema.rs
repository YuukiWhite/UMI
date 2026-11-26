use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotchStyle {
    Attached,
    Floating,
    Glass,
}

impl Default for NotchStyle {
    fn default() -> Self {
        Self::Attached
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpringSpeed {
    Snappy,
    Balanced,
    Calm,
}

impl Default for SpringSpeed {
    fn default() -> Self {
        Self::Balanced
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MascotEyeTint {
    SkyGlint,
    Mint,
    Rose,
    White,
}

impl Default for MascotEyeTint {
    fn default() -> Self {
        Self::SkyGlint
    }
}

impl MascotEyeTint {
    pub fn rgb(self) -> (f64, f64, f64) {
        match self {
            Self::SkyGlint => (0x7D as f64 / 255.0, 0xD3 as f64 / 255.0, 0xFC as f64 / 255.0),
            Self::Mint => (0x34 as f64 / 255.0, 0xD3 as f64 / 255.0, 0x99 as f64 / 255.0),
            Self::Rose => (0xFB as f64 / 255.0, 0x71 as f64 / 255.0, 0x85 as f64 / 255.0),
            Self::White => (1.0, 1.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotchPageId {
    Music,
    Focus,
    Prayer,
    Water,
    Shelf,
    Todo,
    Widgets,
    AskAi,
    AiLimits,
}

impl NotchPageId {
    pub fn title(self) -> &'static str {
        match self {
            Self::Music => "Music",
            Self::Focus => "Focus Timer",
            Self::Prayer => "Prayer",
            Self::Water => "Water",
            Self::Shelf => "Shelf",
            Self::Todo => "TodoList",
            Self::Widgets => "Widgets",
            Self::AskAi => "Ask UMI AI",
            Self::AiLimits => "AI Limits",
        }
    }

    pub fn from_cli_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "music" | "media" => Some(Self::Music),
            "focus" | "pomo" | "pomodoro" => Some(Self::Focus),
            "prayer" | "sholat" | "adzan" => Some(Self::Prayer),
            "water" | "hydration" => Some(Self::Water),
            "shelf" | "files" => Some(Self::Shelf),
            "todo" | "tasks" => Some(Self::Todo),
            "widgets" | "widget" => Some(Self::Widgets),
            "ai" | "ask" | "ask_ai" => Some(Self::AskAi),
            "limits" | "quota" | "ai_limits" => Some(Self::AiLimits),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub start_at_login: bool,
    pub idle_mascot_eyes: bool,
    pub minimal_notch_mode: bool,
    pub show_header_clock: bool,
    pub hide_in_fullscreen: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            start_at_login: false,
            idle_mascot_eyes: true,
            minimal_notch_mode: false,
            show_header_clock: true,
            hide_in_fullscreen: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceConfig {
    pub style: NotchStyle,
    pub scale: f64,
    pub smooth_concave_fillets: bool,
    pub spring_speed: SpringSpeed,
    pub eye_tint: MascotEyeTint,
    pub glass_blur_px: u32,
    pub accent_aura: String,
    pub dim_behind_notch: bool,
    pub display_monitor: Option<String>,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            style: NotchStyle::Attached,
            scale: 1.3,
            smooth_concave_fillets: true,
            spring_speed: SpringSpeed::Balanced,
            eye_tint: MascotEyeTint::SkyGlint,
            glass_blur_px: 24,
            accent_aura: "sky".to_string(),
            dim_behind_notch: false,
            display_monitor: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PagesConfig {
    pub active: Vec<NotchPageId>,
}

impl Default for PagesConfig {
    fn default() -> Self {
        Self {
            active: vec![
                NotchPageId::Music,
                NotchPageId::Focus,
                NotchPageId::Prayer,
                NotchPageId::Water,
                NotchPageId::Shelf,
                NotchPageId::Todo,
                NotchPageId::Widgets,
                NotchPageId::AskAi,
                NotchPageId::AiLimits,
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FocusConfig {
    pub focus_min: u32,
    pub short_break_min: u32,
    pub long_break_min: u32,
    pub sessions_per_round: u32,
    pub auto_start_next: bool,
}

impl Default for FocusConfig {
    fn default() -> Self {
        Self {
            focus_min: 25,
            short_break_min: 5,
            long_break_min: 15,
            sessions_per_round: 4,
            auto_start_next: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WaterConfig {
    pub daily_goal_ml: u32,
    pub bottle_ml: u32,
    pub reminder_interval_min: u32,
    pub reminder_enabled: bool,
    pub gentle_sound: bool,
}

impl Default for WaterConfig {
    fn default() -> Self {
        Self {
            daily_goal_ml: 1500,
            bottle_ml: 250,
            reminder_interval_min: 60,
            reminder_enabled: true,
            gentle_sound: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PrayerConfig {
    pub auto_location: bool,
    pub city: String,
    pub latitude: f64,
    pub longitude: f64,
    pub method: String,
    pub auto_pause_music: bool,
    pub pre_nudge_min: u32,
    pub show_sun_arc: bool,
}

impl Default for PrayerConfig {
    fn default() -> Self {
        Self {
            auto_location: true,
            city: "Surabaya".to_string(),
            latitude: -7.2914,
            longitude: 112.7960,
            method: "kemenag".to_string(),
            auto_pause_music: true,
            pre_nudge_min: 10,
            show_sun_arc: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiConfig {
    /// Active provider: "gemini" | "claude" | "openai" | "ollama"
    pub provider: String,
    pub gemini_model: String,
    pub claude_model: String,
    pub openai_base_url: String,
    pub openai_model: String,
    pub ollama_url: String,
    pub ollama_model: String,
    pub translate_target_lang: String,
    pub low_quota_threshold_pct: u8,
    pub quota_providers: Vec<String>,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: "gemini".to_string(),
            gemini_model: "gemini-2.5-flash".to_string(),
            claude_model: "claude-sonnet-4-5".to_string(),
            openai_base_url: "https://api.openai.com/v1".to_string(),
            openai_model: "gpt-4o-mini".to_string(),
            ollama_url: "http://127.0.0.1:11434".to_string(),
            ollama_model: "llama3.2".to_string(),
            translate_target_lang: "Indonesian".to_string(),
            low_quota_threshold_pct: 20,
            quota_providers: vec!["codex".to_string()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UmiConfig {
    pub general: GeneralConfig,
    pub appearance: AppearanceConfig,
    pub pages: PagesConfig,
    pub focus: FocusConfig,
    pub water: WaterConfig,
    pub prayer: PrayerConfig,
    pub ai: AiConfig,
}

impl Default for UmiConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            appearance: AppearanceConfig::default(),
            pages: PagesConfig::default(),
            focus: FocusConfig::default(),
            water: WaterConfig::default(),
            prayer: PrayerConfig::default(),
            ai: AiConfig::default(),
        }
    }
}
