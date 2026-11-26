use crate::config::NotchPageId;
use std::collections::VecDeque;

/// Four exact size tiers specified in design.figcraft and PRD.md v2.0 (base 1.0x dp).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotchTier {
    Minimal,
    Compact,
    Peek,
    Expanded,
    Wide,
}

#[derive(Debug, Clone, Copy)]
pub struct TierGeometry {
    pub width_dp: f64,
    pub height_dp: f64,
    pub bottom_radius_dp: f64,
    pub ear_radius_dp: f64,
}

impl NotchTier {
    pub fn geometry(self) -> TierGeometry {
        match self {
            Self::Minimal => TierGeometry {
                width_dp: 112.0,
                height_dp: 6.0,
                bottom_radius_dp: 4.0,
                ear_radius_dp: 4.0,
            },
            Self::Compact => TierGeometry {
                width_dp: 150.0,
                height_dp: 30.0,
                bottom_radius_dp: 14.0,
                ear_radius_dp: 10.0,
            },
            Self::Peek => TierGeometry {
                width_dp: 310.0,
                height_dp: 60.0,
                bottom_radius_dp: 19.0,
                ear_radius_dp: 13.0,
            },
            Self::Expanded => TierGeometry {
                width_dp: 420.0,
                height_dp: 140.0,
                bottom_radius_dp: 32.0,
                ear_radius_dp: 22.0,
            },
            Self::Wide => TierGeometry {
                width_dp: 520.0,
                height_dp: 140.0,
                bottom_radius_dp: 32.0,
                ear_radius_dp: 22.0,
            },
        }
    }
}

/// All 18 Notch states from design.figcraft (plus InNotchSettings overlay).
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum NotchState {
    // Compact (01 - 02)
    Resting,
    Closed,
    // Peek Alerts (03 - 05, 18)
    Notification { app: String, sender: String, body: String },
    Headphones { device: String, battery_pct: u8 },
    Charging { battery_pct: u8 },
    QuotaLow { provider: String, remaining_pct: u8, resets_in: String },
    // Expanded (06 - 07, 09 - 17)
    Music,
    FocusTimer,
    Prayer,
    Water,
    InNotchSettings,
    Shelf,
    Clipboard,
    Todo,
    Widgets,
    AiPrompt,
    AiAnswer,
    AiLimits,
    // Wide (08)
    Lyrics,
}

impl NotchState {
    pub fn tier(&self, minimal_mode: bool) -> NotchTier {
        match self {
            Self::Resting | Self::Closed => {
                if minimal_mode {
                    NotchTier::Minimal
                } else {
                    NotchTier::Compact
                }
            }
            Self::Notification { .. }
            | Self::Headphones { .. }
            | Self::Charging { .. }
            | Self::QuotaLow { .. } => NotchTier::Peek,
            Self::Lyrics => NotchTier::Wide,
            _ => NotchTier::Expanded,
        }
    }

    pub fn requires_keyboard(&self) -> bool {
        matches!(self, Self::AiPrompt | Self::AiAnswer | Self::Todo)
    }

    pub fn is_compact(&self) -> bool {
        matches!(self, Self::Resting | Self::Closed)
    }

    pub fn is_expanded_or_wide(&self) -> bool {
        !self.is_compact() && !self.is_peek()
    }

    pub fn is_peek(&self) -> bool {
        matches!(
            self,
            Self::Notification { .. }
                | Self::Headphones { .. }
                | Self::Charging { .. }
                | Self::QuotaLow { .. }
        )
    }

    pub fn from_page(page: NotchPageId) -> Self {
        match page {
            NotchPageId::Music => Self::Music,
            NotchPageId::Focus => Self::FocusTimer,
            NotchPageId::Prayer => Self::Prayer,
            NotchPageId::Water => Self::Water,
            NotchPageId::Shelf => Self::Shelf,
            NotchPageId::Todo => Self::Todo,
            NotchPageId::Widgets => Self::Widgets,
            NotchPageId::AskAi => Self::AiPrompt,
            NotchPageId::AiLimits => Self::AiLimits,
        }
    }

    pub fn status_name(&self) -> &'static str {
        match self {
            Self::Resting => "01-resting",
            Self::Closed => "02-closed",
            Self::Notification { .. } => "03-notification",
            Self::Headphones { .. } => "04-headphones",
            Self::Charging { .. } => "05-charging",
            Self::Music => "06-music",
            Self::FocusTimer => "07-focus",
            Self::Lyrics => "08-lyrics",
            Self::Prayer => "09-prayer",
            Self::Water => "10-water",
            Self::InNotchSettings => "10b-water-settings",
            Self::Shelf => "11-shelf",
            Self::Clipboard => "12-clipboard",
            Self::Todo => "13-todo",
            Self::Widgets => "14-widgets",
            Self::AiPrompt => "15-ai-prompt",
            Self::AiAnswer => "16-ai-answer",
            Self::AiLimits => "17-ai-limits",
            Self::QuotaLow { .. } => "18-quota-low",
        }
    }
}

pub struct StateMachine {
    pub current: NotchState,
    pub pages: Vec<NotchPageId>,
    pub active_page_idx: usize,
    pub idle_mascot_eyes: bool,
    pub media_playing: bool,
    pub interactive_lock: bool,
    pub hidden: bool,
    peek_queue: VecDeque<NotchState>,
}

impl StateMachine {
    pub fn new(pages: Vec<NotchPageId>, idle_mascot_eyes: bool) -> Self {
        let current = if idle_mascot_eyes {
            NotchState::Resting
        } else {
            NotchState::Closed
        };
        Self {
            current,
            pages,
            active_page_idx: 0,
            idle_mascot_eyes,
            media_playing: false,
            interactive_lock: false,
            hidden: false,
            peek_queue: VecDeque::new(),
        }
    }

    pub fn compact_default(&self) -> NotchState {
        if self.media_playing || !self.idle_mascot_eyes {
            NotchState::Closed
        } else {
            NotchState::Resting
        }
    }

    pub fn current_page(&self) -> NotchPageId {
        self.pages
            .get(self.active_page_idx)
            .copied()
            .unwrap_or(NotchPageId::Music)
    }

    pub fn expand(&mut self) {
        self.hidden = false;
        let page = self.current_page();
        self.current = NotchState::from_page(page);
    }

    pub fn collapse(&mut self, force: bool) {
        if self.interactive_lock && !force {
            return;
        }
        self.interactive_lock = false;
        if let Some(next_peek) = self.peek_queue.pop_front() {
            self.current = next_peek;
        } else {
            self.current = self.compact_default();
        }
    }

    pub fn toggle_expand(&mut self) {
        if self.current.is_expanded_or_wide() {
            self.collapse(true);
        } else {
            self.expand();
        }
    }

    pub fn toggle_compact_variant(&mut self) {
        if self.current == NotchState::Resting {
            self.current = NotchState::Closed;
        } else if self.current == NotchState::Closed {
            self.current = NotchState::Resting;
        }
    }

    pub fn next_page(&mut self) {
        if self.pages.is_empty() {
            return;
        }
        self.active_page_idx = (self.active_page_idx + 1) % self.pages.len();
        if self.current.is_expanded_or_wide() {
            self.current = NotchState::from_page(self.current_page());
        }
    }

    pub fn prev_page(&mut self) {
        if self.pages.is_empty() {
            return;
        }
        self.active_page_idx = if self.active_page_idx == 0 {
            self.pages.len() - 1
        } else {
            self.active_page_idx - 1
        };
        if self.current.is_expanded_or_wide() {
            self.current = NotchState::from_page(self.current_page());
        }
    }

    pub fn jump_to_page(&mut self, page: NotchPageId) {
        self.hidden = false;
        if let Some(idx) = self.pages.iter().position(|p| *p == page) {
            self.active_page_idx = idx;
        }
        self.current = NotchState::from_page(page);
    }

    pub fn open_ask_ai(&mut self) {
        self.jump_to_page(NotchPageId::AskAi);
        self.interactive_lock = true;
    }

    pub fn toggle_lyrics(&mut self) {
        if self.current == NotchState::Lyrics {
            self.current = NotchState::Music;
        } else {
            if let Some(idx) = self.pages.iter().position(|p| *p == NotchPageId::Music) {
                self.active_page_idx = idx;
            }
            self.current = NotchState::Lyrics;
        }
    }

    pub fn toggle_shelf_clipboard(&mut self, to_clipboard: bool) {
        if let Some(idx) = self.pages.iter().position(|p| *p == NotchPageId::Shelf) {
            self.active_page_idx = idx;
        }
        self.current = if to_clipboard {
            NotchState::Clipboard
        } else {
            NotchState::Shelf
        };
    }

    pub fn set_media_playing(&mut self, playing: bool) {
        if self.media_playing == playing {
            return;
        }
        self.media_playing = playing;
        if self.current.is_compact() {
            self.current = self.compact_default();
        }
    }

    pub fn enqueue_peek(&mut self, alert: NotchState) {
        if !alert.is_peek() {
            return;
        }
        if self.current.is_compact() {
            self.current = alert;
        } else {
            self.peek_queue.push_back(alert);
        }
    }

    pub fn dismiss_peek(&mut self) {
        if self.current.is_peek() {
            if let Some(next) = self.peek_queue.pop_front() {
                self.current = next;
            } else {
                self.current = self.compact_default();
            }
        }
    }

    pub fn set_mascot_enabled(&mut self, enabled: bool) {
        self.idle_mascot_eyes = enabled;
        if self.current.is_compact() {
            self.current = self.compact_default();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PagesConfig;

    #[test]
    fn state_machine_respects_interactive_lock_and_peek_queue() {
        let mut sm = StateMachine::new(PagesConfig::default().active, true);
        assert_eq!(sm.current, NotchState::Resting);

        sm.open_ask_ai();
        assert_eq!(sm.current, NotchState::AiPrompt);
        assert!(sm.interactive_lock);

        // Normal collapse is blocked while user is typing
        sm.collapse(false);
        assert_eq!(sm.current, NotchState::AiPrompt);

        // Incoming peek queues instead of interrupting typing
        sm.enqueue_peek(NotchState::Charging { battery_pct: 84 });
        assert_eq!(sm.current, NotchState::AiPrompt);

        // Forced collapse (Esc) pops queued peek
        sm.collapse(true);
        assert_eq!(sm.current, NotchState::Charging { battery_pct: 84 });

        // Dismissing peek returns to Resting
        sm.dismiss_peek();
        assert_eq!(sm.current, NotchState::Resting);
    }

    #[test]
    fn pager_wraps_cleanly() {
        let mut sm = StateMachine::new(
            vec![NotchPageId::Music, NotchPageId::Focus, NotchPageId::AiLimits],
            true,
        );
        sm.expand();
        assert_eq!(sm.current, NotchState::Music);
        sm.prev_page();
        assert_eq!(sm.current, NotchState::AiLimits);
        sm.next_page();
        assert_eq!(sm.current, NotchState::Music);
    }
}
