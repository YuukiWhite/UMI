use crate::config::FocusConfig;
use chrono::{Duration, Local};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentKind {
    Focus,
    ShortBreak,
    LongBreak,
}

/// 8-segment Pomodoro engine matching design.figcraft Node 160 (`Focus 1..4` + `Short` + `Long`).
#[derive(Debug, Clone)]
pub struct PomodoroEngine {
    pub cfg: FocusConfig,
    /// Segment index 0..=7 (even = Focus 1..4, odd 1/3/5 = ShortBreak, 7 = LongBreak)
    pub segment_idx: usize,
    pub remaining_sec: f64,
    pub running: bool,
}

impl PomodoroEngine {
    pub fn new(cfg: FocusConfig) -> Self {
        let dur = (cfg.focus_min.max(1) * 60) as f64;
        Self {
            cfg,
            segment_idx: 0,
            remaining_sec: dur,
            running: false,
        }
    }

    pub fn segment_kind(&self, idx: usize) -> SegmentKind {
        if idx >= 7 {
            SegmentKind::LongBreak
        } else if idx % 2 == 0 {
            SegmentKind::Focus
        } else {
            SegmentKind::ShortBreak
        }
    }

    pub fn segment_duration_sec(&self, idx: usize) -> f64 {
        let mins = match self.segment_kind(idx) {
            SegmentKind::Focus => self.cfg.focus_min.max(1),
            SegmentKind::ShortBreak => self.cfg.short_break_min.max(1),
            SegmentKind::LongBreak => self.cfg.long_break_min.max(1),
        };
        (mins * 60) as f64
    }

    pub fn toggle_run(&mut self) {
        self.running = !self.running;
    }

    pub fn reset(&mut self) {
        self.running = false;
        self.segment_idx = 0;
        self.remaining_sec = self.segment_duration_sec(0);
    }

    /// Advance timer by `dt_sec`. Returns a completion message if a segment just finished.
    pub fn tick(&mut self, dt_sec: f64) -> Option<String> {
        if !self.running {
            return None;
        }
        self.remaining_sec -= dt_sec.max(0.0);
        if self.remaining_sec <= 0.0 {
            let finished = self.segment_kind(self.segment_idx);
            self.segment_idx = (self.segment_idx + 1) % 8;
            self.remaining_sec = self.segment_duration_sec(self.segment_idx);
            if !self.cfg.auto_start_next {
                self.running = false;
            }
            return Some(match finished {
                SegmentKind::Focus => "Focus session complete! Time for a break.".to_string(),
                SegmentKind::ShortBreak => "Break finished — back to Focus.".to_string(),
                SegmentKind::LongBreak => "Full Pomodoro round complete!".to_string(),
            });
        }
        None
    }

    pub fn label(&self) -> String {
        let session_num = (self.segment_idx / 2) + 1;
        match self.segment_kind(self.segment_idx) {
            SegmentKind::Focus => format!("Focus · {} of {}", session_num, self.cfg.sessions_per_round),
            SegmentKind::ShortBreak => format!("Short Break · {} of {}", session_num, self.cfg.sessions_per_round),
            SegmentKind::LongBreak => "Long Break".to_string(),
        }
    }

    pub fn remaining_mm_ss(&self) -> String {
        let secs = self.remaining_sec.ceil().max(0.0) as u64;
        format!("{:02}:{:02}", secs / 60, secs % 60)
    }

    pub fn segment_progress(&self) -> f64 {
        let total = self.segment_duration_sec(self.segment_idx).max(1.0);
        (1.0 - self.remaining_sec / total).clamp(0.0, 1.0)
    }

    pub fn estimated_end_hh_mm(&self) -> String {
        let mut total_left = self.remaining_sec.max(0.0) as i64;
        for idx in (self.segment_idx + 1)..8 {
            total_left += self.segment_duration_sec(idx) as i64;
        }
        let end = Local::now() + Duration::seconds(total_left);
        end.format("%H:%M").to_string()
    }

    pub fn round_summary(&self) -> String {
        let total_min = self.cfg.focus_min * 4 + self.cfg.short_break_min * 3 + self.cfg.long_break_min;
        let h = total_min / 60;
        let m = total_min % 60;
        if h > 0 {
            format!("{h} h {m} min · {} sessions", self.cfg.sessions_per_round)
        } else {
            format!("{m} min · {} sessions", self.cfg.sessions_per_round)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pomodoro_advances_through_segments_and_formats_round() {
        let cfg = FocusConfig::default();
        let mut p = PomodoroEngine::new(cfg);
        assert_eq!(p.remaining_mm_ss(), "25:00");
        assert_eq!(p.round_summary(), "2 h 10 min · 4 sessions");
        p.toggle_run();
        let msg = p.tick(25.0 * 60.0 + 1.0);
        assert!(msg.is_some());
        assert_eq!(p.segment_idx, 1);
        assert_eq!(p.segment_kind(1), SegmentKind::ShortBreak);
        assert_eq!(p.remaining_mm_ss(), "05:00");
    }
}
