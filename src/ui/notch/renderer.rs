use crate::config::{NotchPageId, NotchStyle, UmiConfig};
use crate::lyrics::active_line_index;
use crate::state::machine::{NotchState, StateMachine};
use crate::state::spring::NotchSpring;
use crate::quota::QuotaStatus;
use crate::state::{ClipboardEntry, LyricsData, MediaSnapshot};
use crate::ui::notch::phase3::{self, Phase3Data};
use crate::ui::notch::phase4::{self, Phase4Data};
use chrono::{Local, Timelike};
use gtk4::cairo::{self, Context, ImageSurface, LinearGradient};
use std::f64::consts::PI;

#[derive(Clone, Copy)]
pub struct RenderFrame {
    pub notch_x: f64,
    pub notch_w: f64,
    pub notch_h: f64,
    pub bottom_r: f64,
    pub ear_r: f64,
    pub scale: f64,
    pub expanded_mix: f64,
    pub anim_time_sec: f64,
    pub left_arrow_hovered: bool,
    pub right_arrow_hovered: bool,
    pub hovered_dot_idx: Option<usize>,
    pub dot_hover_anim: [f64; 12],
    pub dot_active_anim: [f64; 12],
    pub has_dot_anim: bool,
    pub aura_rgb: [(f64, f64, f64); 3],
    pub eye_tint_rgb: (f64, f64, f64),
    pub has_color_anim: bool,
    pub float_mix: f64,
    pub ear_mix: f64,
    pub glass_mix: f64,
    pub has_style_anim: bool,
    pub mascot_happy_boost: f64,
}

pub struct LiveData<'a> {
    pub battery_pct: u8,
    pub media: &'a MediaSnapshot,
    pub album_art: Option<&'a ImageSurface>,
    pub lyrics: &'a LyricsData,
    pub lyrics_scroll_idx: f64,
    pub clipboard: &'a [ClipboardEntry],
    pub copied_flash_idx: Option<usize>,
    pub phase3: Phase3Data<'a>,
    pub phase4: Phase4Data<'a>,
    pub ai_entry_visible: bool,
}

/// Interactive hit-test regions for Music, Lyrics, and Shelf/Clipboard controls.
#[derive(Debug, Clone, Copy)]
pub struct RectF {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl RectF {
    pub fn contains(self, px: f64, py: f64) -> bool {
        px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
    }
}

pub struct MusicHitZones {
    pub progress_bar: RectF,
    pub lyrics_btn: RectF,
    pub prev_btn: RectF,
    pub play_btn: RectF,
    pub next_btn: RectF,
    pub lyric_prev_line: Option<RectF>,
    pub lyric_next_line: Option<RectF>,
}

pub struct ClipboardHitZones {
    pub shelf_tab: RectF,
    pub clip_tab: RectF,
    pub cards: [RectF; 3],
}

impl RenderFrame {
    pub fn from_state(
        canvas_w: f64,
        _canvas_h: f64,
        spring: &NotchSpring,
        scale: f64,
        anim_time_sec: f64,
        left_arrow_hovered: bool,
        right_arrow_hovered: bool,
    ) -> Self {
        let notch_w = spring.width.value * scale;
        let notch_h = spring.height.value * scale;
        let bottom_r = (spring.bottom_radius.value * scale).min(notch_h * 0.5);
        let ear_r = (spring.ear_radius.value * scale).min(notch_h * 0.65);
        let notch_x = (canvas_w - notch_w) * 0.5;
        Self {
            notch_x,
            notch_w,
            notch_h,
            bottom_r,
            ear_r,
            scale,
            expanded_mix: spring.expanded_mix.value.clamp(0.0, 1.0),
            anim_time_sec,
            left_arrow_hovered,
            right_arrow_hovered,
            hovered_dot_idx: None,
            dot_hover_anim: [0.0; 12],
            dot_active_anim: [0.0; 12],
            has_dot_anim: false,
            aura_rgb: accent_aura_stops("sky"),
            eye_tint_rgb: (0x7D as f64 / 255.0, 0xD3 as f64 / 255.0, 0xFC as f64 / 255.0),
            has_color_anim: false,
            float_mix: 0.0,
            ear_mix: 1.0,
            glass_mix: 0.0,
            has_style_anim: false,
            mascot_happy_boost: 0.0,
        }
    }

    /// Hit-test rectangles for the bottom pager dot bar (generous 14px tall tap/hover target).
    pub fn dot_hit_rects(&self, top_y: f64, count: usize, active_idx: usize) -> Vec<RectF> {
        let unit = self.scale / 1.3;
        let w = 420.0 * self.scale;
        let h = 140.0 * self.scale;
        let x = self.notch_x + (self.notch_w - w) * 0.5;
        let count = count.max(1);
        let active_idx = active_idx.min(count - 1);
        let dot_d = 3.6 * unit;
        let active_w = 15.0 * unit;
        let gap = 5.4 * unit;
        let total_w = (count.saturating_sub(1) as f64) * (dot_d + gap) + active_w;
        let mut dx = x + (w - total_w) * 0.5;
        let dy = top_y + h - 13.0 * unit;

        let mut rects = Vec::with_capacity(count);
        for i in 0..count {
            let item_w = if i == active_idx { active_w } else { dot_d };
            rects.push(RectF {
                x: dx - 2.6 * unit,
                y: dy - 6.0 * unit,
                w: item_w + 5.2 * unit,
                h: dot_d + 12.0 * unit,
            });
            dx += item_w + gap;
        }
        rects
    }

    pub fn top_y(&self, style: NotchStyle) -> f64 {
        let fm = if self.has_style_anim {
            self.float_mix.clamp(0.0, 1.0)
        } else if style == NotchStyle::Floating {
            1.0
        } else {
            0.0
        };
        6.0 * self.scale * fm
    }

    pub fn left_arrow_center(&self) -> (f64, f64) {
        let unit = self.scale / 1.3;
        let target_w = 420.0 * self.scale;
        let x = self.notch_x + (self.notch_w - target_w) * 0.5;
        (x + (22.0 + 11.0) * unit, (82.0 + 11.0) * unit)
    }

    pub fn right_arrow_center(&self) -> (f64, f64) {
        let unit = self.scale / 1.3;
        let target_w = 420.0 * self.scale;
        let x = self.notch_x + (self.notch_w - target_w) * 0.5;
        (x + target_w - (24.0 + 11.0) * unit, (82.0 + 11.0) * unit)
    }

    pub fn arrow_radius(&self) -> f64 {
        11.0 * (self.scale / 1.3)
    }

    pub fn music_hit_zones(&self, wide: bool, top_y: f64) -> MusicHitZones {
        let unit = self.scale / 1.3;
        let target_w = if wide { 520.0 } else { 420.0 } * self.scale;
        let x = self.notch_x + (self.notch_w - target_w) * 0.5;

        if wide {
            MusicHitZones {
                progress_bar: RectF {
                    x: x + 84.0 * unit,
                    y: top_y + 109.0 * unit,
                    w: 140.0 * unit,
                    h: 20.0 * unit,
                },
                lyrics_btn: RectF {
                    x: x + 60.0 * unit,
                    y: top_y + 130.0 * unit,
                    w: 36.0 * unit,
                    h: 32.0 * unit,
                },
                prev_btn: RectF {
                    x: x + 122.0 * unit,
                    y: top_y + 130.0 * unit,
                    w: 36.0 * unit,
                    h: 32.0 * unit,
                },
                play_btn: RectF {
                    x: x + 170.0 * unit,
                    y: top_y + 130.0 * unit,
                    w: 36.0 * unit,
                    h: 32.0 * unit,
                },
                next_btn: RectF {
                    x: x + 216.0 * unit,
                    y: top_y + 130.0 * unit,
                    w: 36.0 * unit,
                    h: 32.0 * unit,
                },
                lyric_prev_line: Some(RectF {
                    x: x + 324.0 * unit,
                    y: top_y + 18.0 * unit,
                    w: 328.0 * unit,
                    h: 34.0 * unit,
                }),
                lyric_next_line: Some(RectF {
                    x: x + 324.0 * unit,
                    y: top_y + 110.0 * unit,
                    w: 328.0 * unit,
                    h: 36.0 * unit,
                }),
            }
        } else {
            MusicHitZones {
                progress_bar: RectF {
                    x: x + 86.0 * unit,
                    y: top_y + 114.0 * unit,
                    w: 342.0 * unit,
                    h: 20.0 * unit,
                },
                lyrics_btn: RectF {
                    x: x + 60.0 * unit,
                    y: top_y + 132.0 * unit,
                    w: 36.0 * unit,
                    h: 32.0 * unit,
                },
                prev_btn: RectF {
                    x: x + 188.0 * unit,
                    y: top_y + 132.0 * unit,
                    w: 36.0 * unit,
                    h: 32.0 * unit,
                },
                play_btn: RectF {
                    x: x + 256.0 * unit,
                    y: top_y + 132.0 * unit,
                    w: 36.0 * unit,
                    h: 32.0 * unit,
                },
                next_btn: RectF {
                    x: x + 324.0 * unit,
                    y: top_y + 132.0 * unit,
                    w: 36.0 * unit,
                    h: 32.0 * unit,
                },
                lyric_prev_line: None,
                lyric_next_line: None,
            }
        }
    }

    pub fn clipboard_hit_zones(&self, top_y: f64) -> ClipboardHitZones {
        let unit = self.scale / 1.3;
        let target_w = 420.0 * self.scale;
        let x = self.notch_x + (self.notch_w - target_w) * 0.5;
        ClipboardHitZones {
            shelf_tab: RectF {
                x: x + 424.0 * unit,
                y: top_y + 8.0 * unit,
                w: 33.0 * unit,
                h: 24.0 * unit,
            },
            clip_tab: RectF {
                x: x + 457.0 * unit,
                y: top_y + 8.0 * unit,
                w: 33.0 * unit,
                h: 24.0 * unit,
            },
            cards: [
                RectF {
                    x: x + 68.0 * unit,
                    y: top_y + 44.0 * unit,
                    w: 94.0 * unit,
                    h: 94.0 * unit,
                },
                RectF {
                    x: x + 172.0 * unit,
                    y: top_y + 44.0 * unit,
                    w: 94.0 * unit,
                    h: 94.0 * unit,
                },
                RectF {
                    x: x + 276.0 * unit,
                    y: top_y + 44.0 * unit,
                    w: 94.0 * unit,
                    h: 94.0 * unit,
                },
            ],
        }
    }
}

/// Draw the full UMI Notch matching design.figcraft across all Phase 1 & Phase 2 states.
pub fn draw_notch(
    cr: &Context,
    frame: &RenderFrame,
    sm: &StateMachine,
    cfg: &UmiConfig,
    live: &LiveData<'_>,
) {
    cr.set_operator(cairo::Operator::Clear);
    let _ = cr.paint();
    cr.set_operator(cairo::Operator::Over);

    if sm.hidden {
        return;
    }

    let x = frame.notch_x;
    let w = frame.notch_w;
    let h = frame.notch_h;
    let br = frame.bottom_r;
    let float_mix = if frame.has_style_anim {
        frame.float_mix.clamp(0.0, 1.0)
    } else if cfg.appearance.style == NotchStyle::Floating {
        1.0
    } else {
        0.0
    };
    let ear_mix = if frame.has_style_anim {
        frame.ear_mix.clamp(0.0, 1.0)
    } else if cfg.appearance.smooth_concave_fillets && cfg.appearance.style != NotchStyle::Floating {
        1.0
    } else {
        0.0
    };
    let glass_mix = if frame.has_style_anim {
        frame.glass_mix.clamp(0.0, 1.0)
    } else if cfg.appearance.style == NotchStyle::Glass {
        1.0
    } else {
        0.0
    };

    // Smoothly retract concave ears before rounding top corners during Attached <-> Floating morph
    let eff_ear = frame.ear_r * ear_mix * (1.0 - float_mix * 2.0).clamp(0.0, 1.0);
    let eff_tr = br.min(h * 0.5) * ((float_mix - 0.18) / 0.82).clamp(0.0, 1.0);
    let top_y = frame.top_y(cfg.appearance.style);

    draw_soft_shadow(cr, x, top_y, w, h, br, eff_tr, 1.0 - 0.65 * glass_mix);

    // Attached/Floating: solid pitch black (#000000, alpha=1.0)
    // Glass: translucent smoky frosted surface + specular white frost sheen + 0.34 rim stroke
    let blur_factor = (cfg.appearance.glass_blur_px as f64 / 40.0).clamp(0.0, 1.0);
    let base_r = 0.065 * glass_mix;
    let base_g = 0.088 * glass_mix;
    let base_b = 0.148 * glass_mix;
    let glass_alpha = 0.42 + 0.16 * blur_factor;
    let body_alpha = 1.0 + (glass_alpha - 1.0) * glass_mix;

    build_notch_path(cr, x, top_y, w, h, br, eff_ear, eff_tr);
    cr.set_source_rgba(base_r, base_g, base_b, body_alpha);
    let _ = cr.fill();

    if glass_mix > 0.01 {
        let frost = LinearGradient::new(x, top_y, x, top_y + h);
        let sheen_boost = 0.75 + 0.45 * blur_factor;
        frost.add_color_stop_rgba(0.0, 1.0, 1.0, 1.0, 0.17 * sheen_boost * glass_mix);
        frost.add_color_stop_rgba(0.45, 0.85, 0.92, 1.0, 0.05 * sheen_boost * glass_mix);
        frost.add_color_stop_rgba(1.0, 0.78, 0.88, 1.0, 0.11 * sheen_boost * glass_mix);
        build_notch_path(cr, x, top_y, w, h, br, eff_ear, eff_tr);
        let _ = cr.set_source(&frost);
        let _ = cr.fill();

        build_notch_path(cr, x, top_y, w, h, br, eff_ear, eff_tr);
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.34 * glass_mix);
        cr.set_line_width(1.2);
        let _ = cr.stroke();
    }

    // Clip inner content strictly to the morphing notch silhouette
    let _ = cr.save();
    build_body_clip_path(cr, x, top_y, w, h, br, eff_tr);
    cr.clip();

    if cfg.general.minimal_notch_mode && sm.current.is_compact() && frame.expanded_mix < 0.05 {
        let [c0, c1, c2] = if frame.has_color_anim {
            frame.aura_rgb
        } else {
            accent_aura_stops(&cfg.appearance.accent_aura)
        };
        let g = LinearGradient::new(x, top_y, x + w, top_y);
        g.add_color_stop_rgba(0.0, c0.0, c0.1, c0.2, 0.45);
        g.add_color_stop_rgba(0.5, c1.0, c1.1, c1.2, 0.70);
        g.add_color_stop_rgba(1.0, c2.0, c2.1, c2.2, 0.45);
        let _ = cr.set_source(&g);
        cr.rectangle(x, top_y + h - 2.0, w, 2.0);
        let _ = cr.fill();
    } else if sm.current.is_peek() {
        draw_peek_shell(cr, frame, top_y, &sm.current);
    } else {
        // Smooth cross-fade + subtle scale between compact and expanded/wide layers
        let raw_c = (1.0 - frame.expanded_mix * 2.2).clamp(0.0, 1.0);
        let compact_alpha = raw_c * raw_c * (3.0 - 2.0 * raw_c);
        if compact_alpha > 0.01 {
            let compact_kind = match sm.current {
                NotchState::Closed => NotchState::Closed,
                NotchState::Resting => NotchState::Resting,
                _ => sm.compact_default(),
            };
            match compact_kind {
                NotchState::Resting => draw_state_01_resting(cr, frame, top_y, cfg, compact_alpha),
                NotchState::Closed => {
                    draw_state_02_closed(cr, frame, top_y, live.battery_pct, live.media.playing, compact_alpha)
                }
                _ => {}
            }
        }

        let raw_e = ((frame.expanded_mix - 0.22) / 0.78).clamp(0.0, 1.0);
        let expanded_alpha = raw_e * raw_e * (3.0 - 2.0 * raw_e);
        if expanded_alpha > 0.01 {
            let _ = cr.save();
            let content_scale = 0.94 + 0.06 * expanded_alpha;
            let pivot_x = frame.notch_x + frame.notch_w * 0.5;
            let pivot_y = top_y + frame.notch_h * 0.35;
            cr.translate(pivot_x, pivot_y);
            cr.scale(content_scale, content_scale);
            cr.translate(-pivot_x, -pivot_y);
            if sm.current == NotchState::Lyrics {
                draw_state_08_lyrics_wide(cr, frame, top_y, live, expanded_alpha);
            } else {
                draw_expanded_shell(cr, frame, top_y, sm, cfg, live, expanded_alpha);
            }
            let _ = cr.restore();
        }
    }

    let _ = cr.restore();
}

fn draw_soft_shadow(cr: &Context, x: f64, y: f64, w: f64, h: f64, br: f64, tr: f64, strength: f64) {
    if strength <= 0.02 {
        return;
    }
    for (spread, alpha) in [(3.5_f64, 0.14_f64), (8.0_f64, 0.07_f64)] {
        build_body_clip_path(
            cr,
            x - spread * 0.3,
            y + spread * 0.45,
            w + spread * 0.6,
            h + spread * 0.4,
            br + spread * 0.25,
            tr,
        );
        cr.set_source_rgba(0.0, 0.0, 0.0, alpha * strength);
        let _ = cr.fill();
    }
}

fn build_notch_path(
    cr: &Context,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    br: f64,
    ear: f64,
    tr: f64,
) {
    if ear <= 0.4 || tr > 0.4 {
        build_body_clip_path(cr, x, y, w, h, br, tr);
        return;
    }

    cr.new_sub_path();
    cr.move_to(x - ear, y);
    cr.arc(x - ear, y + ear, ear, -PI * 0.5, 0.0);
    cr.line_to(x, y + h - br);
    cr.arc_negative(x + br, y + h - br, br, PI, PI * 0.5);
    cr.line_to(x + w - br, y + h);
    cr.arc_negative(x + w - br, y + h - br, br, PI * 0.5, 0.0);
    cr.line_to(x + w, y + ear);
    cr.arc(x + w + ear, y + ear, ear, PI, PI * 1.5);
    cr.close_path();
}

fn build_body_clip_path(
    cr: &Context,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    br: f64,
    tr: f64,
) {
    let tr = tr.clamp(0.0, br.min(h * 0.5));
    cr.new_sub_path();
    if tr > 0.4 {
        cr.arc(x + tr, y + tr, tr, PI, PI * 1.5);
        cr.arc(x + w - tr, y + tr, tr, -PI * 0.5, 0.0);
    } else {
        cr.move_to(x, y);
        cr.line_to(x + w, y);
    }
    cr.arc(x + w - br, y + h - br, br, 0.0, PI * 0.5);
    cr.arc(x + br, y + h - br, br, PI * 0.5, PI);
    cr.close_path();
}

fn draw_rounded_rect(cr: &Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    let r = r.min(w * 0.5).min(h * 0.5);
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -PI * 0.5, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, PI * 0.5);
    cr.arc(x + r, y + h - r, r, PI * 0.5, PI);
    cr.arc(x + r, y + r, r, PI, PI * 1.5);
    cr.close_path();
}

pub fn accent_aura_stops(name: &str) -> [(f64, f64, f64); 3] {
    match name {
        "rose" => [(0.925, 0.282, 0.600), (0.545, 0.361, 0.965), (0.753, 0.518, 0.988)],
        "mint" => [(0.063, 0.725, 0.506), (0.176, 0.831, 0.749), (0.220, 0.741, 0.973)],
        "amber" => [(0.961, 0.620, 0.043), (0.937, 0.267, 0.267), (0.984, 0.443, 0.522)],
        "ocean" => [(0.231, 0.510, 0.965), (0.024, 0.714, 0.831), (0.388, 0.400, 0.945)],
        "mono" => [(0.886, 0.910, 0.941), (0.580, 0.639, 0.722), (0.886, 0.910, 0.941)],
        _ => [(0.220, 0.741, 0.973), (0.388, 0.400, 0.945), (0.659, 0.333, 0.969)], // "sky"
    }
}

fn smoothstep(x: f64) -> f64 {
    let t = x.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn smooth_pulse(t: f64, start: f64, rise: f64, hold: f64, fall: f64) -> f64 {
    let end = start + rise + hold + fall;
    if t <= start || t >= end {
        return 0.0;
    }
    if t < start + rise {
        smoothstep((t - start) / rise)
    } else if t < start + rise + hold {
        1.0
    } else {
        1.0 - smoothstep((t - (start + rise + hold)) / fall)
    }
}

fn blink_pulse(t: f64, start: f64, dur: f64) -> f64 {
    if t <= start || t >= start + dur {
        return 0.0;
    }
    (((t - start) / dur) * PI).sin()
}

/// Returns true only while the procedural mascot expression engine is actively moving,
/// keeping idle Resting CPU usage near 0% between expressions.
pub fn mascot_needs_redraw(anim_time_sec: f64, happy_boost: f64) -> bool {
    if happy_boost > 0.01 {
        return true;
    }
    let t = anim_time_sec.rem_euclid(24.0);
    let active_window = (2.95..=3.28).contains(&t)
        || (4.15..=6.65).contains(&t)
        || (7.55..=8.10).contains(&t)
        || (9.35..=12.25).contains(&t)
        || (13.55..=15.45).contains(&t)
        || (17.15..=19.35).contains(&t)
        || (20.55..=22.75).contains(&t);
    if active_window {
        return true;
    }
    // Gentle sleepy z-drift late at night (23:00 - 05:00)
    let hour = Local::now().hour();
    (hour >= 23 || hour < 5) && (0.4..=2.6).contains(&t)
}

/// State 01 · Resting — expressive procedural UMI mascot (6 animations + night sleepy mode).
fn draw_state_01_resting(
    cr: &Context,
    frame: &RenderFrame,
    top_y: f64,
    cfg: &UmiConfig,
    alpha: f64,
) {
    let x = frame.notch_x;
    let w = frame.notch_w;
    let h = frame.notch_h;
    let unit = frame.scale / 1.3;
    let cx_ref = x + w * 0.5 - 97.5 * unit;

    let [c0, c1, c2] = if frame.has_color_anim {
        frame.aura_rgb
    } else {
        accent_aura_stops(&cfg.appearance.accent_aura)
    };
    let glass_mix = if frame.has_style_anim {
        frame.glass_mix.clamp(0.0, 1.0)
    } else if cfg.appearance.style == NotchStyle::Glass {
        1.0
    } else {
        0.0
    };

    // Draw ambient aura at bottom edge without painting opaque black over Glass mode
    let glow_h = (h * 0.46).max(6.0 * unit);
    let glow_y = top_y + h - glow_h;
    let _ = cr.save();
    cr.rectangle(x, glow_y, w, glow_h);
    cr.clip();
    let glow = LinearGradient::new(x, top_y + h * 0.75, x + w, top_y + h * 0.75);
    glow.add_color_stop_rgba(0.0, c0.0, c0.1, c0.2, 0.78 * alpha);
    glow.add_color_stop_rgba(0.5, c1.0, c1.1, c1.2, 0.88 * alpha);
    glow.add_color_stop_rgba(1.0, c2.0, c2.1, c2.2, 0.78 * alpha);
    let _ = cr.set_source(&glow);
    let v_mask = LinearGradient::new(x, glow_y, x, top_y + h);
    v_mask.add_color_stop_rgba(0.0, 0.0, 0.0, 0.0, 0.0);
    v_mask.add_color_stop_rgba(0.55, 0.0, 0.0, 0.0, 0.35);
    v_mask.add_color_stop_rgba(1.0, 0.0, 0.0, 0.0, 0.92 - 0.15 * glass_mix);
    let _ = cr.mask(&v_mask);
    let _ = cr.restore();

    // 24-second procedural life timeline (6 expressions + reactive joy + night sleepy mode)
    let t = frame.anim_time_sec.rem_euclid(24.0);
    let hour = Local::now().hour();
    let sleepy = hour >= 23 || hour < 5;

    // 1. Natural blink & double-blink
    let b_dur = if sleepy { 0.24 } else { 0.16 };
    let blink_single = blink_pulse(t, 3.0, b_dur);
    let blink_dbl_1 = blink_pulse(t, 7.60, 0.15);
    let blink_dbl_2 = blink_pulse(t, 7.84, 0.15);
    let blink = blink_single.max(blink_dbl_1).max(blink_dbl_2);

    // 2. Glance left & glance right-up
    let glance_left = smooth_pulse(t, 4.2, 0.42, 1.35, 0.55);
    let glance_right_up = smooth_pulse(t, 9.4, 0.45, 1.60, 0.60);
    let look_x = (-4.2 * glance_left + 4.2 * glance_right_up) * unit;
    let look_y = (-1.3 * glance_right_up) * unit;

    // 3. Wink (right eye happy arc)
    let wink = smooth_pulse(t, 13.6, 0.24, 1.05, 0.35);

    // 4. Happy squint (^ ◡ ^) — periodic or reactive boost (after Drink / Task done)
    let happy_cycle = smooth_pulse(t, 17.2, 0.28, 1.35, 0.42);
    let happy = happy_cycle.max(frame.mascot_happy_boost.clamp(0.0, 1.0));

    // 5. Curious / mini yawn (• o •)
    let yawn = smooth_pulse(t, 20.6, 0.40, 1.15, 0.50);

    let (gr, gg, gb) = if frame.has_color_anim {
        frame.eye_tint_rgb
    } else {
        cfg.appearance.eye_tint.rgb()
    };

    // Draw Left & Right Eyes
    for (idx, ex_ref) in [76.0_f64, 108.0_f64].iter().enumerate() {
        let is_right = idx == 1;
        let arc_mix = if is_right { happy.max(wink) } else { happy };
        let squircle_alpha = (1.0 - arc_mix * 1.35).clamp(0.0, 1.0) * alpha;
        let arc_alpha = ((arc_mix - 0.18) / 0.82).clamp(0.0, 1.0) * alpha;

        let base_cx = cx_ref + (ex_ref + 5.5) * unit + look_x;
        let base_cy = top_y + 16.5 * unit + look_y;

        if squircle_alpha > 0.01 {
            let sleepy_lid = if sleepy { 0.72 } else { 1.0 };
            let yawn_scale = 1.0 + 0.08 * yawn;
            let blink_scale = ((1.0 - 0.88 * blink) * sleepy_lid * yawn_scale).clamp(0.12, 1.12);
            let ew = 11.0 * unit * (1.0 + 0.04 * yawn);
            let eh = 13.0 * unit * blink_scale;
            let ex = base_cx - ew * 0.5;
            let ey = base_cy - eh * 0.5 + if sleepy { 1.4 * unit } else { 0.0 };

            draw_rounded_rect(cr, ex, ey, ew, eh, (4.2 * unit).min(eh * 0.5));
            cr.set_source_rgba(1.0, 1.0, 1.0, squircle_alpha);
            let _ = cr.fill();

            if blink_scale > 0.52 {
                let gx = base_cx - 1.5 * unit + look_x * 0.35;
                let gy = ey + (4.0 * unit).min(eh * 0.38) + look_y * 0.35;
                cr.new_sub_path();
                cr.arc(gx, gy, 1.55 * unit, 0.0, 2.0 * PI);
                cr.set_source_rgba(gr, gg, gb, squircle_alpha);
                let _ = cr.fill();
            }
        }

        // Joyful ^ arc eye for Happy Smile & Wink
        if arc_alpha > 0.01 {
            let lift = -1.2 * unit * arc_mix;
            cr.new_sub_path();
            cr.move_to(base_cx - 5.4 * unit, base_cy + 1.8 * unit + lift);
            cr.curve_to(
                base_cx - 2.4 * unit,
                base_cy - 4.4 * unit + lift,
                base_cx + 2.4 * unit,
                base_cy - 4.4 * unit + lift,
                base_cx + 5.4 * unit,
                base_cy + 1.8 * unit + lift,
            );
            cr.set_source_rgba(1.0, 1.0, 1.0, arc_alpha);
            cr.set_line_width(2.2 * unit);
            cr.set_line_cap(cairo::LineCap::Round);
            let _ = cr.stroke();

            // Subtle colored sparkle dot above happy arc
            cr.new_sub_path();
            cr.arc(base_cx, base_cy - 4.8 * unit + lift, 1.2 * unit, 0.0, 2.0 * PI);
            cr.set_source_rgba(gr, gg, gb, 0.78 * arc_alpha);
            let _ = cr.fill();
        }
    }

    // Mouth: smoothly morphs between micro-smile, happy grin, and cute yawn "o"
    let mouth_cx = cx_ref + 97.5 * unit + look_x * 0.32;
    let mouth_cy = top_y + 26.2 * unit + look_y * 0.25;
    let smile_alpha = (1.0 - yawn * 1.4).clamp(0.0, 1.0);
    if smile_alpha > 0.02 {
        let grin = happy.max(wink * 0.65);
        let half_w = (5.5 + 2.2 * grin) * unit;
        let dip = (3.8 + 2.0 * grin) * unit;
        cr.new_sub_path();
        cr.move_to(mouth_cx - half_w, mouth_cy - 0.4 * unit);
        cr.curve_to(
            mouth_cx - half_w * 0.40,
            mouth_cy + dip,
            mouth_cx + half_w * 0.40,
            mouth_cy + dip,
            mouth_cx + half_w,
            mouth_cy - 0.4 * unit,
        );
        cr.set_source_rgba(1.0, 1.0, 1.0, (0.53 + 0.27 * grin) * smile_alpha * alpha);
        cr.set_line_width((1.6 + 0.25 * grin) * unit);
        cr.set_line_cap(cairo::LineCap::Round);
        let _ = cr.stroke();
    }

    if yawn > 0.08 {
        let ya = ((yawn - 0.08) / 0.92).clamp(0.0, 1.0);
        let rx = (1.6 + 1.5 * ya) * unit;
        let ry = (1.8 + 1.9 * ya) * unit;
        let _ = cr.save();
        cr.translate(mouth_cx, mouth_cy + 1.2 * unit);
        cr.scale(rx, ry);
        cr.new_sub_path();
        cr.arc(0.0, 0.0, 1.0, 0.0, 2.0 * PI);
        let _ = cr.restore();
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.68 * ya * alpha);
        cr.set_line_width(1.5 * unit);
        let _ = cr.stroke();
    }

    // Night sleepy tiny drifting "z" (23:00 - 05:00 during calm phase)
    if sleepy && (0.4..=2.6).contains(&t) {
        let p = ((t - 0.4) / 2.2).clamp(0.0, 1.0);
        let za = (p * PI).sin() * 0.55 * alpha;
        cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        cr.set_font_size((7.5 + 2.0 * p) * unit);
        cr.set_source_rgba(gr, gg, gb, za);
        cr.move_to(cx_ref + (132.0 + 6.0 * p) * unit, top_y + (18.0 - 7.0 * p) * unit);
        let _ = cr.show_text("z");
    }
}

/// State 02 · Closed — exact match to design.figcraft Node 140 (195x39 @ 1.3x).
fn draw_state_02_closed(
    cr: &Context,
    frame: &RenderFrame,
    top_y: f64,
    battery_pct: u8,
    playing: bool,
    alpha: f64,
) {
    let unit = frame.scale / 1.3;
    let compact_w = 195.0 * unit;
    let compact_h = 39.0 * unit;
    let x = frame.notch_x + (frame.notch_w - compact_w) * 0.5;

    draw_battery_ring(
        cr,
        x + 31.5 * unit,
        top_y + 19.5 * unit,
        6.2 * unit,
        2.0 * unit,
        battery_pct,
        alpha,
    );

    let time_str = Local::now().format("%H:%M").to_string();
    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(14.5 * unit);
    if let Ok(ext) = cr.text_extents(&time_str) {
        let tx = x + (compact_w - ext.width()) * 0.5 - ext.x_bearing();
        let ty = top_y + (compact_h - ext.height()) * 0.5 - ext.y_bearing();
        cr.move_to(tx, ty);
        cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
        let _ = cr.show_text(&time_str);
    }

    draw_live_eq(
        cr,
        x + 152.0 * unit,
        top_y + 19.5 * unit,
        unit,
        if playing { frame.anim_time_sec } else { 0.0 },
        playing,
        alpha,
    );
}

fn draw_battery_ring(
    cr: &Context,
    cx: f64,
    cy: f64,
    radius: f64,
    line_width: f64,
    pct: u8,
    alpha: f64,
) {
    cr.set_line_width(line_width);
    cr.set_line_cap(cairo::LineCap::Round);
    cr.new_sub_path();
    cr.arc(cx, cy, radius, 0.0, 2.0 * PI);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.16 * alpha);
    let _ = cr.stroke();

    let frac = (pct as f64 / 100.0).clamp(0.05, 1.0);
    let start = -PI * 0.5;
    cr.new_sub_path();
    cr.arc(cx, cy, radius, start, start + frac * 2.0 * PI);
    if pct <= 20 {
        cr.set_source_rgba(0.961, 0.620, 0.043, alpha);
    } else {
        cr.set_source_rgba(0.180, 0.859, 0.420, alpha); // #2EDB6B
    }
    let _ = cr.stroke();
}

fn draw_live_eq(
    cr: &Context,
    start_x: f64,
    center_y: f64,
    unit: f64,
    t: f64,
    animated: bool,
    alpha: f64,
) {
    let base_heights = [6.0_f64, 11.0, 14.0, 9.0, 6.0];
    let phases = [0.0_f64, 1.1, 2.3, 3.6, 4.8];
    cr.set_source_rgba(1.0, 1.0, 1.0, if animated { alpha } else { 0.45 * alpha });

    for i in 0..5 {
        let bh = if animated {
            let wave = 0.45 + 0.55 * (t * 5.2 + phases[i]).sin().abs();
            (base_heights[i] * wave).max(3.5) * unit
        } else {
            (base_heights[i] * 0.55).max(3.5) * unit
        };
        let bw = 2.2 * unit;
        let bx = start_x + (i as f64) * 4.2 * unit;
        let by = center_y - bh * 0.5;
        draw_rounded_rect(cr, bx, by, bw, bh, bw * 0.5);
        let _ = cr.fill();
    }
}

/// Peek States (03 Notification, 04 Headphones, 05 Charging, 18 QuotaLow) — 1:1 FigCraft 403x78 @ 1.3x.
fn draw_peek_shell(cr: &Context, frame: &RenderFrame, top_y: f64, state: &NotchState) {
    let unit = frame.scale / 1.3;
    let w = 310.0 * frame.scale;
    let x = frame.notch_x + (frame.notch_w - w) * 0.5;

    match state {
        NotchState::Notification { app, sender, body } => {
            // 03 · Notification: #25D366 rounded-square + white chat bubble
            draw_rounded_rect(cr, x + 24.0 * unit, top_y + 21.0 * unit, 36.0 * unit, 36.0 * unit, 10.0 * unit);
            cr.set_source_rgb(0.145, 0.827, 0.400);
            let _ = cr.fill();

            // Chat bubble glyph inside green badge
            cr.new_sub_path();
            cr.arc(x + 42.0 * unit, top_y + 38.0 * unit, 8.5 * unit, 0.0, 2.0 * PI);
            cr.set_source_rgb(1.0, 1.0, 1.0);
            let _ = cr.fill();
            cr.new_sub_path();
            cr.move_to(x + 35.5 * unit, top_y + 43.0 * unit);
            cr.line_to(x + 34.0 * unit, top_y + 48.0 * unit);
            cr.line_to(x + 40.0 * unit, top_y + 45.5 * unit);
            cr.close_path();
            let _ = cr.fill();

            cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
            cr.set_font_size(11.0 * unit);
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.45);
            cr.move_to(x + 72.0 * unit, top_y + 26.0 * unit);
            let _ = cr.show_text(&truncate_text(app, 24));

            cr.move_to(x + 355.0 * unit, top_y + 26.0 * unit);
            let _ = cr.show_text("now");

            cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
            cr.set_font_size(14.0 * unit);
            cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
            cr.move_to(x + 72.0 * unit, top_y + 44.5 * unit);
            let _ = cr.show_text(&truncate_text(sender, 28));

            cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
            cr.set_font_size(12.0 * unit);
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.66);
            cr.move_to(x + 72.0 * unit, top_y + 62.5 * unit);
            let _ = cr.show_text(&truncate_text(body, 38));
        }
        NotchState::Headphones { device, battery_pct } => {
            // 04 · Headphones: #5EC2FF ring + Bluetooth rune + right #0D1A26 headphones badge
            let cx = x + 41.0 * unit;
            let cy = top_y + 39.0 * unit;
            draw_colored_ring(cr, cx, cy, 16.0 * unit, 2.8 * unit, *battery_pct, (0.369, 0.761, 1.0));
            draw_bluetooth_glyph(cr, cx, cy, unit);

            cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
            cr.set_font_size(15.0 * unit);
            cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
            cr.move_to(x + 72.0 * unit, top_y + 36.0 * unit);
            let _ = cr.show_text(&truncate_text(device, 22));

            cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
            cr.set_font_size(12.0 * unit);
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.53);
            cr.move_to(x + 72.0 * unit, top_y + 56.0 * unit);
            let _ = cr.show_text(&format!("Connected · {battery_pct}%"));

            // Right headphone badge (335, 18, 42x42)
            let bx = x + 356.0 * unit;
            let by = top_y + 39.0 * unit;
            cr.new_sub_path();
            cr.arc(bx, by, 21.0 * unit, 0.0, 2.0 * PI);
            cr.set_source_rgb(0.051, 0.102, 0.149); // #0D1A26
            let _ = cr.fill();
            cr.new_sub_path();
            cr.arc(bx, by, 16.0 * unit, 0.0, 2.0 * PI);
            cr.set_source_rgba(0.369, 0.761, 1.0, 0.35);
            cr.set_line_width(1.2 * unit);
            let _ = cr.stroke();
            draw_headphones_glyph(cr, bx, by, unit);
        }
        NotchState::Charging { battery_pct } => {
            // 05 · Charging: #33E07A ring + white bolt + large right percentage
            let cx = x + 41.0 * unit;
            let cy = top_y + 39.0 * unit;
            draw_colored_ring(cr, cx, cy, 16.0 * unit, 2.8 * unit, *battery_pct, (0.200, 0.878, 0.478));
            draw_bolt_glyph(cr, cx, cy, unit);

            cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
            cr.set_font_size(15.0 * unit);
            cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
            cr.move_to(x + 72.0 * unit, top_y + 36.0 * unit);
            let _ = cr.show_text("Charging");

            cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
            cr.set_font_size(12.0 * unit);
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.47);
            cr.move_to(x + 72.0 * unit, top_y + 56.0 * unit);
            let _ = cr.show_text("Charger connected");

            let pct_str = format!("{battery_pct}%");
            cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
            cr.set_font_size(25.0 * unit);
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.60);
            if let Ok(ext) = cr.text_extents(&pct_str) {
                cr.move_to(x + w - 26.0 * unit - ext.width(), top_y + 48.0 * unit);
                let _ = cr.show_text(&pct_str);
            }
        }
        NotchState::QuotaLow {
            provider,
            remaining_pct,
            resets_in,
        } => {
            let cx = x + 41.0 * unit;
            let cy = top_y + 39.0 * unit;
            draw_colored_ring(cr, cx, cy, 16.0 * unit, 2.8 * unit, *remaining_pct, (0.961, 0.620, 0.043));
            cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
            cr.set_font_size(14.0 * unit);
            cr.set_source_rgb(0.961, 0.620, 0.043);
            cr.move_to(cx - 2.5 * unit, cy + 5.0 * unit);
            let _ = cr.show_text("!");

            cr.set_font_size(14.5 * unit);
            cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
            cr.move_to(x + 72.0 * unit, top_y + 36.0 * unit);
            let _ = cr.show_text(&format!("{provider} · Quota Low"));

            cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
            cr.set_font_size(11.5 * unit);
            cr.set_source_rgba(0.961, 0.620, 0.043, 0.85);
            cr.move_to(x + 72.0 * unit, top_y + 56.0 * unit);
            let _ = cr.show_text(&format!("{remaining_pct}% remaining · Resets in {resets_in}"));

            let pct_str = format!("{remaining_pct}%");
            cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
            cr.set_font_size(23.0 * unit);
            cr.set_source_rgb(0.961, 0.620, 0.043);
            if let Ok(ext) = cr.text_extents(&pct_str) {
                cr.move_to(x + w - 26.0 * unit - ext.width(), top_y + 48.0 * unit);
                let _ = cr.show_text(&pct_str);
            }
        }
        _ => {}
    }
}

fn draw_colored_ring(cr: &Context, cx: f64, cy: f64, r: f64, lw: f64, pct: u8, rgb: (f64, f64, f64)) {
    cr.set_line_width(lw);
    cr.set_line_cap(cairo::LineCap::Round);
    cr.new_sub_path();
    cr.arc(cx, cy, r, 0.0, 2.0 * PI);
    cr.set_source_rgba(rgb.0, rgb.1, rgb.2, 0.18);
    let _ = cr.stroke();
    let frac = (pct as f64 / 100.0).clamp(0.08, 1.0);
    cr.new_sub_path();
    cr.arc(cx, cy, r, -PI * 0.5, -PI * 0.5 + frac * 2.0 * PI);
    cr.set_source_rgb(rgb.0, rgb.1, rgb.2);
    let _ = cr.stroke();
}

fn draw_bolt_glyph(cr: &Context, cx: f64, cy: f64, unit: f64) {
    cr.new_sub_path();
    cr.move_to(cx + 1.5 * unit, cy - 7.5 * unit);
    cr.line_to(cx - 4.8 * unit, cy + 0.8 * unit);
    cr.line_to(cx - 0.5 * unit, cy + 0.8 * unit);
    cr.line_to(cx - 1.8 * unit, cy + 7.5 * unit);
    cr.line_to(cx + 4.8 * unit, cy - 1.0 * unit);
    cr.line_to(cx + 0.5 * unit, cy - 1.0 * unit);
    cr.close_path();
    cr.set_source_rgb(1.0, 1.0, 1.0);
    let _ = cr.fill();
}

fn draw_bluetooth_glyph(cr: &Context, cx: f64, cy: f64, unit: f64) {
    cr.new_sub_path();
    cr.move_to(cx - 3.5 * unit, cy - 3.5 * unit);
    cr.line_to(cx + 3.5 * unit, cy + 3.2 * unit);
    cr.line_to(cx, cy + 6.5 * unit);
    cr.line_to(cx, cy - 6.5 * unit);
    cr.line_to(cx + 3.5 * unit, cy - 3.2 * unit);
    cr.line_to(cx - 3.5 * unit, cy + 3.5 * unit);
    cr.set_source_rgb(0.369, 0.761, 1.0);
    cr.set_line_width(1.6 * unit);
    cr.set_line_cap(cairo::LineCap::Round);
    cr.set_line_join(cairo::LineJoin::Round);
    let _ = cr.stroke();
}

fn draw_headphones_glyph(cr: &Context, cx: f64, cy: f64, unit: f64) {
    cr.new_sub_path();
    cr.arc(cx, cy + 1.0 * unit, 6.8 * unit, PI, 2.0 * PI);
    cr.set_source_rgb(0.369, 0.761, 1.0);
    cr.set_line_width(1.7 * unit);
    let _ = cr.stroke();
    draw_rounded_rect(cr, cx - 8.2 * unit, cy + 0.5 * unit, 3.2 * unit, 6.2 * unit, 1.4 * unit);
    let _ = cr.fill();
    draw_rounded_rect(cr, cx + 5.0 * unit, cy + 0.5 * unit, 3.2 * unit, 6.2 * unit, 1.4 * unit);
    let _ = cr.fill();
}

/// Shared Expanded Shell matching design.figcraft (546x182 @ 1.3x / 420x140 dp).
fn draw_expanded_shell(
    cr: &Context,
    frame: &RenderFrame,
    top_y: f64,
    sm: &StateMachine,
    cfg: &UmiConfig,
    live: &LiveData<'_>,
    alpha: f64,
) {
    let unit = frame.scale / 1.3;
    let w = 420.0 * frame.scale;
    let h = 140.0 * frame.scale;
    let x = frame.notch_x + (frame.notch_w - w) * 0.5;
    let page = sm.current_page();
    let is_clipboard = sm.current == NotchState::Clipboard;
    let is_water_settings = sm.current == NotchState::InNotchSettings;

    let stable = RenderFrame {
        notch_x: x,
        notch_w: w,
        notch_h: h,
        ..*frame
    };
    let frame = &stable;

    if is_water_settings {
        phase3::draw_in_notch_water_settings(cr, x, top_y, unit, cfg, alpha);
        return;
    }

    if sm.current == NotchState::AiAnswer {
        phase4::draw_state_16_ai_answer(cr, x, top_y, w, h, unit, &live.phase4, alpha);
        let (lx, ly) = frame.left_arrow_center();
        let (rx, ry) = frame.right_arrow_center();
        let ar = frame.arrow_radius();
        draw_pager_arrow(cr, lx - 10.0 * unit, top_y + ly, ar * 0.88, true, frame.left_arrow_hovered, alpha * 0.9);
        draw_pager_arrow(cr, rx + 10.0 * unit, top_y + ry, ar * 0.88, false, frame.right_arrow_hovered, alpha * 0.9);
        draw_bottom_dots(cr, x, top_y, w, h, unit, sm, frame, alpha);
        return;
    }

    if page == NotchPageId::AskAi {
        phase4::draw_state_15_ai_prompt(cr, x, top_y, w, h, unit, &live.phase4, live.ai_entry_visible, alpha);
    }

    // 1. Header Row (Card 06 Music omits left title, matching FigCraft)
    if page != NotchPageId::Music {
        cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        cr.set_font_size(13.5 * unit);
        match page {
            NotchPageId::Water => {
                cr.set_source_rgba(0.925, 0.937, 0.969, alpha);
                cr.move_to(x + 24.0 * unit, top_y + 23.0 * unit);
                let _ = cr.show_text("Water ");
                let w1 = cr.text_extents("Water ").map(|e| e.x_advance()).unwrap_or(44.0 * unit);
                cr.set_source_rgba(0.333, 0.784, 1.0, alpha); // #55C8FF
                cr.move_to(x + 24.0 * unit + w1, top_y + 23.0 * unit);
                let _ = cr.show_text("Reminder");
            }
            NotchPageId::Todo => {
                cr.set_source_rgba(0.925, 0.937, 0.969, alpha);
                cr.move_to(x + 24.0 * unit, top_y + 23.0 * unit);
                let _ = cr.show_text("Todo");
                let w1 = cr.text_extents("Todo").map(|e| e.x_advance()).unwrap_or(36.0 * unit);
                cr.set_source_rgba(0.216, 0.827, 0.435, alpha); // #37D36F
                cr.move_to(x + 24.0 * unit + w1, top_y + 23.0 * unit);
                let _ = cr.show_text("List");
            }
            NotchPageId::AskAi => {
                cr.set_source_rgba(0.925, 0.937, 0.969, alpha);
                cr.move_to(x + 42.0 * unit, top_y + 23.0 * unit);
                let _ = cr.show_text("Ask ");
                let w_ask = cr.text_extents("Ask ").map(|e| e.x_advance()).unwrap_or(28.0 * unit);
                cr.set_source_rgba(0.490, 0.827, 0.988, alpha);
                cr.move_to(x + 42.0 * unit + w_ask, top_y + 23.0 * unit);
                let _ = cr.show_text("UMI AI");
            }
            NotchPageId::AiLimits => {
                cr.set_source_rgba(0.925, 0.937, 0.969, alpha);
                cr.move_to(x + 24.0 * unit, top_y + 23.0 * unit);
                let _ = cr.show_text("AI ");
                let w_ai = cr.text_extents("AI ").map(|e| e.x_advance()).unwrap_or(18.0 * unit);
                cr.set_source_rgba(0.490, 0.827, 0.988, alpha);
                cr.move_to(x + 24.0 * unit + w_ai, top_y + 23.0 * unit);
                let _ = cr.show_text("Limits");
            }
            _ => {
                cr.set_source_rgba(0.925, 0.937, 0.969, alpha);
                cr.move_to(x + 24.0 * unit, top_y + 23.0 * unit);
                let _ = cr.show_text(page.title());
            }
        }
    }

    if cfg.general.show_header_clock {
        let now = Local::now();
        let date_str = now.format("%a %d %b %Y").to_string();
        let time_str = now.format("%H:%M").to_string();

        let center_x = x + w * 0.5;
        draw_battery_ring(
            cr,
            center_x - 82.0 * unit,
            top_y + 18.5 * unit,
            6.2 * unit,
            1.8 * unit,
            live.battery_pct,
            alpha,
        );

        cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
        cr.set_font_size(12.0 * unit);
        cr.set_source_rgba(0.75, 0.79, 0.87, 0.85 * alpha);
        cr.move_to(center_x - 66.0 * unit, top_y + 22.5 * unit);
        let _ = cr.show_text(&date_str);

        cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        cr.set_font_size(12.0 * unit);
        cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
        cr.move_to(center_x + 42.0 * unit, top_y + 22.5 * unit);
        let _ = cr.show_text(&time_str);
    }

    // Header right: Shelf|Clipboard switcher, "N active" pill, or 5-bar EQ (Card 15 Ask AI has no right EQ)
    if page == NotchPageId::Shelf {
        draw_shelf_clipboard_switcher(cr, x + 424.0 * unit, top_y + 10.0 * unit, unit, is_clipboard, alpha);
    } else if page == NotchPageId::AiLimits {
        let active_n = live
            .phase4
            .quotas
            .iter()
            .filter(|q| q.status != QuotaStatus::Unavailable)
            .count();
        let px = x + 458.0 * unit;
        let py = top_y + 10.0 * unit;
        draw_rounded_rect(cr, px, py, 64.0 * unit, 18.0 * unit, 9.0 * unit);
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.07 * alpha);
        let _ = cr.fill();
        cr.new_sub_path();
        cr.arc(px + 10.5 * unit, py + 9.0 * unit, 2.5 * unit, 0.0, 2.0 * PI);
        cr.set_source_rgba(0.180, 0.859, 0.420, alpha);
        let _ = cr.fill();
        cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
        cr.set_font_size(10.0 * unit);
        cr.set_source_rgba(0.796, 0.831, 0.882, alpha);
        cr.move_to(px + 18.0 * unit, py + 12.5 * unit);
        let _ = cr.show_text(&format!("{active_n} active"));
    } else if page != NotchPageId::AskAi {
        draw_live_eq(
            cr,
            x + w - 50.0 * unit,
            top_y + 18.5 * unit,
            unit,
            frame.anim_time_sec,
            live.media.playing,
            alpha,
        );
    }

    // 2. Pager Arrow Buttons (always accessible including Ask UMI AI)
    let (lx, ly) = frame.left_arrow_center();
    let (rx, ry) = frame.right_arrow_center();
    let ar = frame.arrow_radius();
    if page == NotchPageId::AskAi {
        draw_pager_arrow(cr, lx - 10.0 * unit, top_y + ly, ar * 0.88, true, frame.left_arrow_hovered, alpha * 0.9);
        draw_pager_arrow(cr, rx + 10.0 * unit, top_y + ry, ar * 0.88, false, frame.right_arrow_hovered, alpha * 0.9);
    } else {
        if page != NotchPageId::Music || frame.left_arrow_hovered {
            draw_pager_arrow(cr, lx, top_y + ly, ar, true, frame.left_arrow_hovered, alpha);
        }
        draw_pager_arrow(cr, rx, top_y + ry, ar, false, frame.right_arrow_hovered, alpha);
    }

    // 3. Page Content Body
    match page {
        NotchPageId::Music => draw_state_06_music(cr, frame, top_y, live, alpha),
        NotchPageId::Focus => phase3::draw_state_07_focus(cr, x, top_y, unit, &live.phase3, alpha),
        NotchPageId::Prayer => phase3::draw_state_09_prayer(cr, x, top_y, unit, &live.phase3, alpha),
        NotchPageId::Water => phase3::draw_state_10_water(cr, x, top_y, unit, cfg, &live.phase3, alpha),
        NotchPageId::Shelf if is_clipboard => draw_state_12_clipboard(cr, frame, top_y, live, alpha),
        NotchPageId::Shelf => phase3::draw_state_11_shelf(cr, x, top_y, unit, &live.phase3, alpha),
        NotchPageId::Todo => phase3::draw_state_13_todo(cr, x, top_y, unit, &live.phase3, alpha),
        NotchPageId::Widgets => phase3::draw_state_14_widgets(cr, x, top_y, unit, &live.phase3, alpha),
        NotchPageId::AskAi => {} // Already drawn beneath header glow
        NotchPageId::AiLimits => phase4::draw_state_17_ai_limits(cr, x, top_y, unit, &live.phase4, alpha),
    }

    // 4. Bottom Dot Indicator Bar (animated hoverable & clickable pills)
    draw_bottom_dots(cr, x, top_y, w, h, unit, sm, frame, alpha);
}

fn draw_bottom_dots(
    cr: &Context,
    x: f64,
    top_y: f64,
    w: f64,
    h: f64,
    unit: f64,
    sm: &StateMachine,
    frame: &RenderFrame,
    alpha: f64,
) {
    let count = sm.pages.len().max(1);
    let active_idx = sm.active_page_idx.min(count - 1);
    let dot_d = 3.6 * unit;
    let active_w = 15.0 * unit;
    let gap = 5.4 * unit;
    let dy = top_y + h - 13.0 * unit;

    let mut widths = [0.0_f64; 12];
    let mut active_ts = [0.0_f64; 12];
    let mut hover_ts = [0.0_f64; 12];
    let mut total_w = 0.0;

    for i in 0..count.min(12) {
        let a_t = if frame.has_dot_anim {
            frame.dot_active_anim[i].clamp(0.0, 1.0)
        } else if i == active_idx {
            1.0
        } else {
            0.0
        };
        let h_t = if frame.has_dot_anim {
            frame.dot_hover_anim[i].clamp(0.0, 1.25)
        } else if frame.hovered_dot_idx == Some(i) {
            1.0
        } else {
            0.0
        };
        active_ts[i] = a_t;
        hover_ts[i] = h_t;
        // Morph smoothly between circle dot_d and active pill active_w, plus subtle hover swell
        let w_i = dot_d
            + (active_w - dot_d) * a_t
            + 2.2 * unit * h_t.max(0.0) * (1.0 - a_t * 0.7);
        widths[i] = w_i;
        total_w += w_i;
    }
    total_w += (count.min(12).saturating_sub(1) as f64) * gap;

    let mut dx = x + (w - total_w) * 0.5;

    for i in 0..count.min(12) {
        let a_t = active_ts[i];
        let h_t = hover_ts[i];
        let h_clamped = h_t.clamp(0.0, 1.0);
        let item_w = widths[i];

        let pill_h = dot_d * (1.0 + 0.42 * h_t);
        let lift_y = -0.75 * unit * h_t;
        let cy = dy + dot_d * 0.5 + lift_y;
        let py = cy - pill_h * 0.5;

        // Soft accent glow halo behind hovered dot (follows smooth Accent Aura)
        let (ar_c, ag_c, ab_c) = frame.aura_rgb[0];
        let hi_r = (ar_c + 0.25).min(1.0);
        let hi_g = (ag_c + 0.14).min(1.0);
        let hi_b = (ab_c + 0.12).min(1.0);
        if h_clamped > 0.02 {
            let glow_pad = (2.6 + 1.4 * h_clamped) * unit;
            draw_rounded_rect(
                cr,
                dx - glow_pad * 0.65,
                py - glow_pad * 0.65,
                item_w + glow_pad * 1.3,
                pill_h + glow_pad * 1.3,
                (pill_h + glow_pad * 1.3) * 0.5,
            );
            cr.set_source_rgba(ar_c, ag_c, ab_c, 0.26 * h_clamped * alpha);
            let _ = cr.fill();
        }

        // Smooth color blend: inactive white(0.30) -> active white(1.0) -> accent hover highlight
        let base_a = 0.30 + 0.70 * a_t;
        let r = 1.0 + (hi_r - 1.0) * h_clamped;
        let g = 1.0 + (hi_g - 1.0) * h_clamped;
        let b = 1.0 + (hi_b - 1.0) * h_clamped;
        let final_a = (base_a + (1.0 - base_a) * h_clamped).clamp(0.0, 1.0) * alpha;

        draw_rounded_rect(cr, dx, py, item_w, pill_h, pill_h * 0.5);
        cr.set_source_rgba(r, g, b, final_a);
        let _ = cr.fill();

        dx += item_w + gap;
    }
}

/// State 06 · Music — 1:1 FigCraft Card 06 (546x182 @ 1.3x).
fn draw_state_06_music(cr: &Context, frame: &RenderFrame, top_y: f64, live: &LiveData<'_>, alpha: f64) {
    let x = frame.notch_x;
    let unit = frame.scale / 1.3;

    draw_album_art(cr, x + 36.0 * unit, top_y + 38.0 * unit, 62.0 * unit, live.album_art, alpha);

    let title = if live.media.has_track() {
        live.media.title.as_str()
    } else {
        "No Media Playing"
    };
    let artist = if !live.media.artist.is_empty() {
        live.media.artist.as_str()
    } else if live.media.has_track() {
        "Unknown Artist"
    } else {
        "Play Spotify, YouTube, or YouTube Music"
    };

    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(14.5 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
    cr.move_to(x + 114.0 * unit, top_y + 58.0 * unit);
    let _ = cr.show_text(&truncate_text(title, 34));

    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(12.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.45 * alpha);
    cr.move_to(x + 114.0 * unit, top_y + 78.0 * unit);
    let _ = cr.show_text(&truncate_text(artist, 40));

    // Elapsed / Seekable Progress Bar / Remaining (y = 118..127)
    let pos_us = live.media.live_position_us();
    let dur_us = live.media.duration_us;
    let ratio = if dur_us > 0 {
        (pos_us as f64 / dur_us as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let elapsed_s = format_mm_ss(pos_us / 1_000_000);
    let rem_s = if dur_us > 0 {
        format!("-{}", format_mm_ss((dur_us - pos_us).max(0) / 1_000_000))
    } else {
        "-00:00".to_string()
    };

    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(11.5 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.85 * alpha);
    cr.move_to(x + 36.0 * unit, top_y + 128.0 * unit);
    let _ = cr.show_text(&elapsed_s);

    draw_rounded_rect(cr, x + 86.0 * unit, top_y + 123.0 * unit, 342.0 * unit, 4.0 * unit, 2.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.22 * alpha);
    let _ = cr.fill();
    if ratio > 0.005 {
        draw_rounded_rect(
            cr,
            x + 86.0 * unit,
            top_y + 123.0 * unit,
            (342.0 * ratio).max(4.0) * unit,
            4.0 * unit,
            2.0 * unit,
        );
        cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
        let _ = cr.fill();
    }

    cr.set_source_rgba(1.0, 1.0, 1.0, 0.85 * alpha);
    cr.move_to(x + 444.0 * unit, top_y + 128.0 * unit);
    let _ = cr.show_text(&rem_s);

    // Bottom Controls: Lyrics Bubble (70, 142), Prev (198, 142), Play/Pause (267.5, 141.5), Next (334, 142)
    draw_lyrics_bubble_icon(cr, x + 70.0 * unit, top_y + 141.0 * unit, unit, false, alpha);
    draw_prev_icon(cr, x + 198.0 * unit, top_y + 142.0 * unit, unit, alpha);
    draw_play_pause_icon(cr, x + 267.5 * unit, top_y + 141.5 * unit, unit, live.media.playing, alpha);
    draw_next_icon(cr, x + 334.0 * unit, top_y + 142.0 * unit, unit, alpha);
}

/// State 08 · Lyrics (Wide 520x140 dp / 676x182 px @ 1.3x) — 1:1 FigCraft Card 08.
fn draw_state_08_lyrics_wide(
    cr: &Context,
    frame: &RenderFrame,
    top_y: f64,
    live: &LiveData<'_>,
    alpha: f64,
) {
    let unit = frame.scale / 1.3;
    let w = 520.0 * frame.scale;
    let h = 140.0 * frame.scale;
    let x = frame.notch_x + (frame.notch_w - w) * 0.5;

    // Blue Ambient Backdrop (#2C64B8 -> #3979D0 -> #2A5AA8) + 3-stop vertical black fade
    let bg = LinearGradient::new(x, top_y + 100.0 * unit, x + w, top_y + 100.0 * unit);
    bg.add_color_stop_rgba(0.0, 0.173, 0.392, 0.722, alpha);
    bg.add_color_stop_rgba(0.5, 0.224, 0.475, 0.816, alpha);
    bg.add_color_stop_rgba(1.0, 0.165, 0.353, 0.659, alpha);
    let _ = cr.set_source(&bg);
    cr.rectangle(x, top_y + 26.0 * unit, w, h - 26.0 * unit);
    let _ = cr.fill();

    let mask = LinearGradient::new(x + w * 0.5, top_y, x + w * 0.5, top_y + h);
    mask.add_color_stop_rgba(0.0, 0.0, 0.0, 0.0, 0.98 * alpha);
    mask.add_color_stop_rgba(0.45, 0.0, 0.0, 0.0, 0.67 * alpha);
    mask.add_color_stop_rgba(1.0, 0.0, 0.0, 0.0, 0.20 * alpha);
    let _ = cr.set_source(&mask);
    cr.rectangle(x, top_y, w, h);
    let _ = cr.fill();

    // Left Split Pane: Album Art + Track + Compact Progress + Controls
    draw_album_art(cr, x + 34.0 * unit, top_y + 34.0 * unit, 62.0 * unit, live.album_art, alpha);

    let title = if live.media.has_track() {
        live.media.title.as_str()
    } else {
        "No Media Playing"
    };
    let artist = if !live.media.artist.is_empty() {
        live.media.artist.as_str()
    } else {
        "Spotify · YouTube · YTM"
    };

    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(14.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
    cr.move_to(x + 112.0 * unit, top_y + 54.0 * unit);
    let _ = cr.show_text(&truncate_text(title, 21));

    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(11.5 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.52 * alpha);
    cr.move_to(x + 112.0 * unit, top_y + 74.0 * unit);
    let _ = cr.show_text(&truncate_text(artist, 25));

    let pos_us = live.media.live_position_us();
    let dur_us = live.media.duration_us;
    let ratio = if dur_us > 0 {
        (pos_us as f64 / dur_us as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let elapsed_s = format_mm_ss(pos_us / 1_000_000);
    let rem_s = if dur_us > 0 {
        format!("-{}", format_mm_ss((dur_us - pos_us).max(0) / 1_000_000))
    } else {
        "-00:00".to_string()
    };

    cr.set_font_size(11.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.85 * alpha);
    cr.move_to(x + 34.0 * unit, top_y + 122.0 * unit);
    let _ = cr.show_text(&elapsed_s);

    draw_rounded_rect(cr, x + 84.0 * unit, top_y + 117.0 * unit, 140.0 * unit, 4.0 * unit, 2.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.22 * alpha);
    let _ = cr.fill();
    if ratio > 0.005 {
        draw_rounded_rect(
            cr,
            x + 84.0 * unit,
            top_y + 117.0 * unit,
            (140.0 * ratio).max(4.0) * unit,
            4.0 * unit,
            2.0 * unit,
        );
        cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
        let _ = cr.fill();
    }

    cr.set_source_rgba(1.0, 1.0, 1.0, 0.85 * alpha);
    cr.move_to(x + 236.0 * unit, top_y + 122.0 * unit);
    let _ = cr.show_text(&rem_s);

    draw_lyrics_bubble_icon(cr, x + 70.0 * unit, top_y + 139.0 * unit, unit, true, alpha);
    draw_prev_icon(cr, x + 132.0 * unit, top_y + 140.0 * unit, unit, alpha);
    draw_play_pause_icon(cr, x + 181.0 * unit, top_y + 139.5 * unit, unit, live.media.playing, alpha);
    draw_next_icon(cr, x + 226.0 * unit, top_y + 140.0 * unit, unit, alpha);

    // Center Vertical Divider (306, 36, 1x110)
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.14 * alpha);
    cr.rectangle(x + 306.0 * unit, top_y + 36.0 * unit, 1.0 * unit, 110.0 * unit);
    let _ = cr.fill();

    // Right Split Pane: Subpixel-smooth Apple Music style gliding lyrics
    // Uses HintMetrics::Off + continuous cr.scale() + consistent wrapping so text never snaps or jerks.
    let lx = x + 332.0 * unit;
    let pane_w = (w - 350.0 * unit).max(40.0);
    let _ = cr.save();
    cr.rectangle(lx - 8.0 * unit, top_y + 12.0 * unit, pane_w + 16.0 * unit, h - 24.0 * unit);
    cr.clip();

    if let Ok(mut fopts) = cairo::FontOptions::new() {
        fopts.set_antialias(cairo::Antialias::Gray);
        fopts.set_hint_style(cairo::HintStyle::None);
        fopts.set_hint_metrics(cairo::HintMetrics::Off);
        cr.set_font_options(&fopts);
    }

    match live.lyrics {
        LyricsData::Synced(lines) if !lines.is_empty() => {
            let pos_ms = (pos_us / 1000).max(0) as u64;
            let active_idx = active_line_index(lines, pos_ms).unwrap_or(0);
            let scroll = if live.lyrics_scroll_idx.is_finite() {
                live.lyrics_scroll_idx.clamp(0.0, (lines.len() - 1) as f64)
            } else {
                active_idx as f64
            };

            let center_y = top_y + 90.0 * unit;
            let line_step = 50.0 * unit;
            let base = scroll.floor() as isize;
            let base_font_sz = 16.2 * unit;

            cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
            cr.set_font_size(base_font_sz);

            for offset in -2..=3 {
                let i = base + offset;
                if i < 0 || i as usize >= lines.len() {
                    continue;
                }
                let text = lines[i as usize].text.as_str();
                let delta = i as f64 - scroll;
                let dist = delta.abs();
                let focus_raw = (1.0 - dist.min(1.0)).clamp(0.0, 1.0);
                let focus = focus_raw * focus_raw * (3.0 - 2.0 * focus_raw);
                let edge_fade = (1.0 - ((dist - 1.15).max(0.0) / 0.75)).clamp(0.0, 1.0);
                let line_alpha = (0.28 + 0.72 * focus) * edge_fade * alpha;
                if line_alpha <= 0.01 {
                    continue;
                }

                let y = center_y + delta * line_step;
                // Continuous subpixel scale transform (0.83 -> 1.0) with zero font-hinting staircase
                let s = 0.83 + 0.17 * focus;
                let (l1, l2) = wrap_two_lines(text, 34);

                let _ = cr.save();
                cr.translate(lx, y);
                cr.scale(s, s);
                cr.set_source_rgba(1.0, 1.0, 1.0, line_alpha);
                if let Some(second) = l2 {
                    cr.move_to(0.0, -5.0 * unit);
                    let _ = cr.show_text(&l1);
                    cr.move_to(0.0, 14.5 * unit);
                    let _ = cr.show_text(&second);
                } else {
                    cr.move_to(0.0, 5.0 * unit);
                    let _ = cr.show_text(&l1);
                }
                let _ = cr.restore();
            }
        }
        LyricsData::Plain(lines) if !lines.is_empty() => {
            for (i, line) in lines.iter().take(3).enumerate() {
                cr.select_font_face(
                    "Inter",
                    cairo::FontSlant::Normal,
                    if i == 1 { cairo::FontWeight::Bold } else { cairo::FontWeight::Normal },
                );
                cr.set_font_size((if i == 1 { 15.5 } else { 13.5 }) * unit);
                cr.set_source_rgba(1.0, 1.0, 1.0, (if i == 1 { 0.95 } else { 0.40 }) * alpha);
                cr.move_to(lx, top_y + (44.0 + i as f64 * 42.0) * unit);
                let _ = cr.show_text(&truncate_text(line, 36));
            }
        }
        LyricsData::Loading => {
            cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
            cr.set_font_size(15.0 * unit);
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.85 * alpha);
            cr.move_to(lx, top_y + 92.0 * unit);
            let _ = cr.show_text("Fetching synced lyrics via LRCLIB...");
        }
        _ => {
            cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
            cr.set_font_size(15.0 * unit);
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.72 * alpha);
            cr.move_to(lx, top_y + 84.0 * unit);
            let _ = cr.show_text("No synced lyrics found");
            cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
            cr.set_font_size(12.0 * unit);
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.38 * alpha);
            cr.move_to(lx, top_y + 108.0 * unit);
            let _ = cr.show_text("Auto-syncs with Spotify, YouTube & YouTube Music");
        }
    }
    let _ = cr.restore();
}

/// State 12 · Clipboard — 1:1 FigCraft Card 12 (546x182 @ 1.3x).
fn draw_state_12_clipboard(cr: &Context, frame: &RenderFrame, top_y: f64, live: &LiveData<'_>, alpha: f64) {
    let x = frame.notch_x;
    let unit = frame.scale / 1.3;
    let now = chrono::Utc::now().timestamp();

    let xs = [68.0_f64, 172.0, 276.0];
    for (idx, cx_ref) in xs.iter().enumerate() {
        let cx = x + cx_ref * unit;
        let cy = top_y + 44.0 * unit;
        let cw = 94.0 * unit;
        let ch = 94.0 * unit;
        let is_copied = live.copied_flash_idx == Some(idx);

        draw_rounded_rect(cr, cx, cy, cw, ch, 12.0 * unit);
        if is_copied {
            cr.set_source_rgba(0.180, 0.859, 0.420, 0.22 * alpha);
        } else {
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.07 * alpha);
        }
        let _ = cr.fill();

        if let Some(entry) = live.clipboard.get(idx) {
            let (l1, l2) = wrap_two_lines(&entry.text, 13);
            cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
            cr.set_font_size(10.5 * unit);
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.85 * alpha);
            cr.move_to(cx + 9.0 * unit, cy + 20.0 * unit);
            let _ = cr.show_text(&l1);
            if let Some(line2) = l2 {
                cr.move_to(cx + 9.0 * unit, cy + 35.0 * unit);
                let _ = cr.show_text(&line2);
            }

            let age = if is_copied {
                "Copied!".to_string()
            } else {
                format_relative_age(now - entry.captured_at)
            };
            cr.set_font_size(9.5 * unit);
            if is_copied {
                cr.set_source_rgba(0.200, 0.878, 0.478, alpha);
            } else {
                cr.set_source_rgba(1.0, 1.0, 1.0, 0.40 * alpha);
            }
            cr.move_to(cx + 9.0 * unit, cy + 84.0 * unit);
            let _ = cr.show_text(&age);
        } else {
            cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
            cr.set_font_size(9.5 * unit);
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.22 * alpha);
            cr.move_to(cx + 14.0 * unit, cy + 50.0 * unit);
            let _ = cr.show_text("Empty slot");
        }
    }

    let footer = format!("{} items · click to copy", live.clipboard.len());
    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(10.5 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.40 * alpha);
    cr.move_to(x + 68.0 * unit, top_y + 154.0 * unit);
    let _ = cr.show_text(&footer);
}

fn draw_shelf_clipboard_switcher(
    cr: &Context,
    sx: f64,
    sy: f64,
    unit: f64,
    clipboard_active: bool,
    alpha: f64,
) {
    draw_rounded_rect(cr, sx, sy, 66.0 * unit, 20.0 * unit, 10.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.10 * alpha);
    let _ = cr.fill();

    let thumb_x = if clipboard_active {
        sx + 32.0 * unit
    } else {
        sx + 2.0 * unit
    };
    draw_rounded_rect(cr, thumb_x, sy + 2.0 * unit, 32.0 * unit, 16.0 * unit, 8.0 * unit);
    cr.set_source_rgba(0.361, 0.329, 1.0, alpha); // #5C54FF
    let _ = cr.fill();

    // Folder icon on left
    let fx = sx + 12.0 * unit;
    let fy = sy + 6.5 * unit;
    draw_rounded_rect(cr, fx, fy, 10.0 * unit, 7.5 * unit, 1.5 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, (if clipboard_active { 0.45 } else { 1.0 }) * alpha);
    cr.set_line_width(1.2 * unit);
    let _ = cr.stroke();

    // Clipboard icon on right
    let cx = sx + 45.0 * unit;
    let cy = sy + 5.5 * unit;
    draw_rounded_rect(cr, cx, cy, 8.0 * unit, 9.5 * unit, 1.5 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, (if clipboard_active { 1.0 } else { 0.45 }) * alpha);
    cr.set_line_width(1.2 * unit);
    let _ = cr.stroke();
}

/// Renders either real cached MPRIS2 album artwork or exact FigCraft blue vinyl + 6 staff lines fallback.
fn draw_album_art(
    cr: &Context,
    x: f64,
    y: f64,
    size: f64,
    art: Option<&ImageSurface>,
    alpha: f64,
) {
    let _ = cr.save();
    draw_rounded_rect(cr, x, y, size, size, size * 0.19);
    cr.clip();

    if let Some(surface) = art {
        let sw = surface.width().max(1) as f64;
        let sh = surface.height().max(1) as f64;
        let scale = (size / sw).max(size / sh);
        let ox = x + (size - sw * scale) * 0.5;
        let oy = y + (size - sh * scale) * 0.5;
        cr.translate(ox, oy);
        cr.scale(scale, scale);
        let _ = cr.set_source_surface(surface, 0.0, 0.0);
        let _ = cr.paint_with_alpha(alpha);
    } else {
        // FigCraft Node 230 exact fallback: #E4ECF7 sheet + 6 #B8CCE7 staff lines + blue vinyl disc
        cr.set_source_rgba(0.894, 0.925, 0.969, alpha);
        cr.rectangle(x, y, size, size);
        let _ = cr.fill();

        cr.set_source_rgba(0.722, 0.800, 0.906, alpha);
        for i in 0..6 {
            let sy = y + (10.0 + i as f64 * 8.0) * (size / 62.0);
            cr.rectangle(x, sy, size, 1.0);
            let _ = cr.fill();
        }

        let cx = x + size * 0.5;
        let cy = y + size * 0.5;
        let r = size * (21.0 / 62.0);
        let vg = LinearGradient::new(cx - r, cy - r, cx + r, cy + r);
        vg.add_color_stop_rgba(0.0, 0.145, 0.388, 0.922, alpha);
        vg.add_color_stop_rgba(0.7, 0.110, 0.306, 0.847, alpha);
        vg.add_color_stop_rgba(1.0, 0.231, 0.510, 0.965, alpha);
        cr.new_sub_path();
        cr.arc(cx, cy, r, 0.0, 2.0 * PI);
        let _ = cr.set_source(&vg);
        let _ = cr.fill();

        cr.new_sub_path();
        cr.arc(cx, cy, size * (15.0 / 62.0), 0.0, 2.0 * PI);
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.22 * alpha);
        cr.set_line_width(1.0);
        let _ = cr.stroke();

        cr.new_sub_path();
        cr.arc(cx, cy, size * (6.0 / 62.0), 0.0, 2.0 * PI);
        cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
        let _ = cr.fill();
    }

    let _ = cr.restore();
}

fn draw_lyrics_bubble_icon(cr: &Context, x: f64, y: f64, unit: f64, active: bool, alpha: f64) {
    draw_rounded_rect(cr, x, y, 16.0 * unit, 12.0 * unit, 3.0 * unit);
    if active {
        cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
        let _ = cr.fill();
    } else {
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.72 * alpha);
        cr.set_line_width(1.4 * unit);
        let _ = cr.stroke();
    }
    cr.new_sub_path();
    cr.move_to(x + 3.5 * unit, y + 12.0 * unit);
    cr.line_to(x + 2.5 * unit, y + 15.2 * unit);
    cr.line_to(x + 7.0 * unit, y + 12.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, (if active { 1.0 } else { 0.72 }) * alpha);
    let _ = cr.fill();
}

fn draw_prev_icon(cr: &Context, x: f64, y: f64, unit: f64, alpha: f64) {
    cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
    for dx in [0.0, 8.0] {
        cr.new_sub_path();
        cr.move_to(x + (dx + 8.0) * unit, y);
        cr.line_to(x + dx * unit, y + 6.0 * unit);
        cr.line_to(x + (dx + 8.0) * unit, y + 12.0 * unit);
        cr.close_path();
        let _ = cr.fill();
    }
}

fn draw_next_icon(cr: &Context, x: f64, y: f64, unit: f64, alpha: f64) {
    cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
    for dx in [0.0, 8.0] {
        cr.new_sub_path();
        cr.move_to(x + dx * unit, y);
        cr.line_to(x + (dx + 8.0) * unit, y + 6.0 * unit);
        cr.line_to(x + dx * unit, y + 12.0 * unit);
        cr.close_path();
        let _ = cr.fill();
    }
}

fn draw_play_pause_icon(cr: &Context, x: f64, y: f64, unit: f64, playing: bool, alpha: f64) {
    cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
    if playing {
        draw_rounded_rect(cr, x + 1.0 * unit, y, 4.0 * unit, 13.0 * unit, 1.2 * unit);
        let _ = cr.fill();
        draw_rounded_rect(cr, x + 8.0 * unit, y, 4.0 * unit, 13.0 * unit, 1.2 * unit);
        let _ = cr.fill();
    } else {
        cr.new_sub_path();
        cr.move_to(x + 2.0 * unit, y);
        cr.line_to(x + 13.0 * unit, y + 6.5 * unit);
        cr.line_to(x + 2.0 * unit, y + 13.0 * unit);
        cr.close_path();
        let _ = cr.fill();
    }
}

fn draw_pager_arrow(
    cr: &Context,
    cx: f64,
    cy: f64,
    r: f64,
    is_left: bool,
    hovered: bool,
    alpha: f64,
) {
    cr.new_sub_path();
    cr.arc(cx, cy, r, 0.0, 2.0 * PI);
    let bg_a = if hovered { 0.24 } else { 0.14 };
    cr.set_source_rgba(1.0, 1.0, 1.0, bg_a * alpha);
    let _ = cr.fill();

    let dir = if is_left { -1.0 } else { 1.0 };
    cr.new_sub_path();
    cr.move_to(cx - dir * r * 0.16, cy - r * 0.34);
    cr.line_to(cx + dir * r * 0.22, cy);
    cr.line_to(cx - dir * r * 0.16, cy + r * 0.34);
    cr.set_source_rgba(1.0, 1.0, 1.0, (if hovered { 0.98 } else { 0.78 }) * alpha);
    cr.set_line_width(1.6 * (r / 11.0));
    cr.set_line_cap(cairo::LineCap::Round);
    cr.set_line_join(cairo::LineJoin::Round);
    let _ = cr.stroke();
}

fn format_mm_ss(total_sec: i64) -> String {
    let s = total_sec.max(0);
    format!("{:02}:{:02}", s / 60, s % 60)
}

fn format_relative_age(delta_sec: i64) -> String {
    if delta_sec < 60 {
        "just now".to_string()
    } else if delta_sec < 3600 {
        format!("{} min ago", delta_sec / 60)
    } else {
        format!("{} h ago", delta_sec / 3600)
    }
}

fn truncate_text(s: &str, max_chars: usize) -> String {
    let mut chars = s.chars();
    let head: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{}…", head.trim_end())
    } else {
        head
    }
}

fn wrap_two_lines(s: &str, line_chars: usize) -> (String, Option<String>) {
    let clean = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.chars().count() <= line_chars {
        return (clean, None);
    }
    let mut line1 = String::new();
    let mut rest = String::new();
    for word in clean.split_whitespace() {
        if rest.is_empty() && (line1.len() + word.len() + 1 <= line_chars || line1.is_empty()) {
            if !line1.is_empty() {
                line1.push(' ');
            }
            line1.push_str(word);
        } else {
            if !rest.is_empty() {
                rest.push(' ');
            }
            rest.push_str(word);
        }
    }
    if rest.is_empty() {
        (truncate_text(&line1, line_chars), None)
    } else {
        (truncate_text(&line1, line_chars), Some(truncate_text(&rest, line_chars)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::AiResponse;
    use crate::config::UmiConfig;
    use crate::domain::pomodoro::PomodoroEngine;
    use crate::domain::prayer_calc::compute_schedule_for_date;
    use crate::lyrics::LyricLine;
    use crate::quota::QuotaSnapshot;
    use crate::storage::WaterSummary;

    #[test]
    fn renders_phase4_figcraft_states_to_ppm() {
        let scale = 1.3;
        let canvas_w = 700;
        let row_heights = [224, 224, 224, 110];
        let total_h: i32 = row_heights.iter().sum::<i32>() + 40;

        let mut surface = cairo::ImageSurface::create(cairo::Format::ARgb32, canvas_w, total_h)
            .expect("create image surface");
        let cr = Context::new(&surface).expect("create cairo context");
        cr.set_source_rgb(0.039, 0.051, 0.094);
        let _ = cr.paint();

        let mut cfg = UmiConfig::default();
        cfg.appearance.scale = scale;

        let pomo = PomodoroEngine::new(cfg.focus.clone());
        let prayer = compute_schedule_for_date("Surabaya", -7.2914, 112.7960, "kemenag", 2026, 11, 26, 7.0, 14 * 60 + 30);
        let water = WaterSummary::default();
        let sample_media = MediaSnapshot::default();
        let sample_lyrics = LyricsData::Synced(vec![LyricLine { time_ms: 0, text: "test".into() }]);
        let sample_ai = AiResponse {
            prompt_echo: "You: Summarize today's design sprint".into(),
            answer: "• Finalized UMI Tapered Notch logo across all sizes\n• Built 7 colorful Settings screens & tray quick-menu\n• Ready to ship Pure Rust Phase 4".into(),
            latency_ms: 420,
            provider_label: "UMI AI".into(),
            auto_copy: false,
        };
        let quotas = vec![
            QuotaSnapshot {
                provider_id: "antigravity".into(),
                display_name: "Antigravity".into(),
                plan_label: "Pro · 5h window".into(),
                remaining_pct: 78,
                resets_in_sec: Some(2 * 3600 + 15 * 60),
                status: QuotaStatus::Ok,
                accent_hex: "#38BDF8".into(),
                updated_at: 0,
            },
            QuotaSnapshot {
                provider_id: "codex".into(),
                display_name: "Codex".into(),
                plan_label: "Plus · Weekly".into(),
                remaining_pct: 42,
                resets_in_sec: Some(3 * 86_400 + 4 * 3600),
                status: QuotaStatus::Ok,
                accent_hex: "#10B981".into(),
                updated_at: 0,
            },
            QuotaSnapshot {
                provider_id: "claude".into(),
                display_name: "Claude".into(),
                plan_label: "Max · Session".into(),
                remaining_pct: 18,
                resets_in_sec: Some(48 * 60),
                status: QuotaStatus::Low,
                accent_hex: "#F59E0B".into(),
                updated_at: 0,
            },
        ];

        let live = LiveData {
            battery_pct: 84,
            media: &sample_media,
            album_art: None,
            lyrics: &sample_lyrics,
            lyrics_scroll_idx: 0.0,
            clipboard: &[],
            copied_flash_idx: None,
            phase3: Phase3Data {
                pomodoro: &pomo,
                prayer: &prayer,
                water: &water,
                shelf: &[],
                shelf_drag_over: false,
                tasks: &[],
                todo_edit_mode: false,
                todo_scroll: 0,
                battery_pct: 84,
                battery_charging: true,
            },
            phase4: Phase4Data {
                translate_mode: false,
                ai_loading: false,
                ai_response: Some(&sample_ai),
                ai_copied_flash: false,
                quotas: &quotas,
            },
            ai_entry_visible: false,
        };

        let states: [(NotchState, f64); 4] = [
            (NotchState::AiPrompt, 1.0),
            (NotchState::AiAnswer, 1.0),
            (NotchState::AiLimits, 1.0),
            (
                NotchState::QuotaLow {
                    provider: "Claude".into(),
                    remaining_pct: 18,
                    resets_in: "48m".into(),
                },
                0.0,
            ),
        ];

        let mut y_off = 0.0;
        for (idx, (st, mix)) in states.iter().enumerate() {
            let rh = row_heights[idx] as f64;
            let _ = cr.save();
            cr.translate(0.0, y_off);

            let sub = cairo::ImageSurface::create(cairo::Format::ARgb32, canvas_w, rh as i32)
                .expect("sub");
            let sub_cr = Context::new(&sub).expect("sub cr");

            let mut sm = StateMachine::new(cfg.pages.active.clone(), true);
            match st {
                NotchState::AiPrompt => sm.jump_to_page(NotchPageId::AskAi),
                NotchState::AiAnswer => {
                    sm.jump_to_page(NotchPageId::AskAi);
                    sm.current = NotchState::AiAnswer;
                }
                NotchState::AiLimits => sm.jump_to_page(NotchPageId::AiLimits),
                _ => sm.current = st.clone(),
            }

            let geom = sm.current.tier(false).geometry();
            let mut spring = NotchSpring::new(
                geom.width_dp,
                geom.height_dp,
                geom.bottom_radius_dp,
                geom.ear_radius_dp,
            );
            spring.expanded_mix.value = *mix;
            let frame = RenderFrame::from_state(canvas_w as f64, rh, &spring, scale, 1.2, false, false);
            draw_notch(&sub_cr, &frame, &sm, &cfg, &live);

            let bg = LinearGradient::new(0.0, 0.0, 0.0, rh);
            bg.add_color_stop_rgb(0.0, 0.859, 0.906, 1.0);
            bg.add_color_stop_rgb(0.45, 0.718, 0.804, 1.0);
            bg.add_color_stop_rgb(1.0, 0.494, 0.584, 1.0);
            let _ = cr.set_source(&bg);
            cr.rectangle(0.0, 0.0, canvas_w as f64, rh);
            let _ = cr.fill();

            let _ = cr.set_source_surface(&sub, 0.0, 0.0);
            let _ = cr.paint();

            let _ = cr.restore();
            y_off += rh + 10.0;
        }

        drop(cr);
        surface.flush();
        let stride = surface.stride() as usize;
        let data = surface.data().expect("data");
        let mut out = format!("P6\n{canvas_w} {total_h}\n255\n").into_bytes();
        out.reserve((canvas_w * total_h * 3) as usize);
        for y in 0..(total_h as usize) {
            for x in 0..(canvas_w as usize) {
                let off = y * stride + x * 4;
                out.push(data[off + 2]);
                out.push(data[off + 1]);
                out.push(data[off]);
            }
        }
        std::fs::write("/tmp/umi-phase4-render.ppm", &out).expect("write phase4 ppm");
    }

    #[test]
    fn renders_readme_showcase_and_mascot_expressions() {
        let save_ppm = |mut surface: cairo::ImageSurface, path: &str| {
            surface.flush();
            let w = surface.width() as usize;
            let h = surface.height() as usize;
            let stride = surface.stride() as usize;
            let data = surface.data().expect("surface data");
            let mut out = format!("P6\n{w} {h}\n255\n").into_bytes();
            out.reserve(w * h * 3);
            for y in 0..h {
                for x in 0..w {
                    let off = y * stride + x * 4;
                    out.push(data[off + 2]);
                    out.push(data[off + 1]);
                    out.push(data[off]);
                }
            }
            std::fs::write(path, &out).expect("write ppm");
        };

        let mut cfg = UmiConfig::default();
        cfg.appearance.scale = 1.3;
        let pomo = PomodoroEngine::new(cfg.focus.clone());
        let prayer = compute_schedule_for_date("Surabaya", -7.2914, 112.7960, "kemenag", 2026, 11, 26, 7.0, 14 * 60 + 30);
        let water = WaterSummary {
            today_ml: 1250,
            hourly_ml: [250, 0, 250, 250, 0, 250, 250, 0],
        };
        let tasks = vec![
            crate::storage::TaskItem { id: 1, title: "Polish UMI mascot expressions".into(), completed: true },
            crate::storage::TaskItem { id: 2, title: "Ship Pure Rust Wayland release".into(), completed: true },
            crate::storage::TaskItem { id: 3, title: "Record demo screenshots for README".into(), completed: false },
        ];
        let media = MediaSnapshot {
            bus_name: Some("spotify".into()),
            title: "Blue Horizon".into(),
            artist: "Summer Coast".into(),
            playing: true,
            duration_us: 214_000_000,
            position_us: 82_000_000,
            position_synced_at: None,
            art_path: None,
        };
        let lyrics = LyricsData::Synced(vec![
            LyricLine { time_ms: 74_000, text: "Drifting over the quiet midnight sea".into() },
            LyricLine { time_ms: 80_000, text: "Chasing starlight across the blue horizon".into() },
            LyricLine { time_ms: 88_000, text: "Every moment glowing soft and bright".into() },
        ]);
        let quotas = vec![
            QuotaSnapshot {
                provider_id: "codex_5h".into(),
                display_name: "Codex 5h".into(),
                plan_label: "Plus · 5h window".into(),
                remaining_pct: 84,
                resets_in_sec: Some(3 * 3600 + 22 * 60),
                status: QuotaStatus::Ok,
                accent_hex: "#10B981".into(),
                updated_at: 0,
            },
            QuotaSnapshot {
                provider_id: "codex_weekly".into(),
                display_name: "Codex Weekly".into(),
                plan_label: "Plus · Weekly".into(),
                remaining_pct: 71,
                resets_in_sec: Some(4 * 86_400 + 6 * 3600),
                status: QuotaStatus::Ok,
                accent_hex: "#10B981".into(),
                updated_at: 0,
            },
        ];
        let live = LiveData {
            battery_pct: 84,
            media: &media,
            album_art: None,
            lyrics: &lyrics,
            lyrics_scroll_idx: 1.0,
            clipboard: &[],
            copied_flash_idx: None,
            phase3: Phase3Data {
                pomodoro: &pomo,
                prayer: &prayer,
                water: &water,
                shelf: &[],
                shelf_drag_over: false,
                tasks: &tasks,
                todo_edit_mode: false,
                todo_scroll: 0,
                battery_pct: 84,
                battery_charging: true,
            },
            phase4: Phase4Data {
                translate_mode: false,
                ai_loading: false,
                ai_response: None,
                ai_copied_flash: false,
                quotas: &quotas,
            },
            ai_entry_visible: false,
        };

        // 1. Mascot Expressions Strip (5 expressions side by side)
        {
            let bw = 1280_i32;
            let bh = 160_i32;
            let surf = cairo::ImageSurface::create(cairo::Format::ARgb32, bw, bh).expect("mascot surf");
            let cr = Context::new(&surf).expect("cr");
            let bg = LinearGradient::new(0.0, 0.0, bw as f64, bh as f64);
            bg.add_color_stop_rgb(0.03, 0.043, 0.063, 0.125);
            bg.add_color_stop_rgb(0.5, 0.067, 0.098, 0.196);
            bg.add_color_stop_rgb(1.0, 0.043, 0.063, 0.125);
            let _ = cr.set_source(&bg);
            let _ = cr.paint();

            let exprs: [(&str, f64); 5] = [
                ("Idle", 1.0),
                ("Glance Left", 5.0),
                ("Wink", 14.3),
                ("Happy", 18.0),
                ("Curious / Yawn", 21.4),
            ];
            let sm = StateMachine::new(cfg.pages.active.clone(), true);
            let geom = NotchState::Resting.tier(false).geometry();
            let spring = NotchSpring::new(geom.width_dp, geom.height_dp, geom.bottom_radius_dp, geom.ear_radius_dp);

            let cell_w = bw as f64 / exprs.len() as f64;
            for (i, (label, ts)) in exprs.iter().enumerate() {
                let cx = i as f64 * cell_w;
                // Mini desktop top strip backdrop
                draw_rounded_rect(&cr, cx + 14.0, 22.0, cell_w - 28.0, 88.0, 14.0);
                let card_bg = LinearGradient::new(cx, 22.0, cx, 110.0);
                card_bg.add_color_stop_rgb(0.0, 0.78, 0.85, 0.98);
                card_bg.add_color_stop_rgb(1.0, 0.45, 0.56, 0.78);
                let _ = cr.set_source(&card_bg);
                let _ = cr.fill();

                let sub = cairo::ImageSurface::create(cairo::Format::ARgb32, (cell_w - 28.0) as i32, 70).expect("sub");
                let sub_cr = Context::new(&sub).expect("sub_cr");
                let frame = RenderFrame::from_state(cell_w - 28.0, 70.0, &spring, 1.15, *ts, false, false);
                draw_notch(&sub_cr, &frame, &sm, &cfg, &live);
                let _ = cr.set_source_surface(&sub, cx + 14.0, 22.0);
                let _ = cr.paint();

                cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
                cr.set_font_size(13.0);
                cr.set_source_rgba(0.92, 0.94, 0.98, 0.92);
                if let Ok(ext) = cr.text_extents(label) {
                    cr.move_to(cx + (cell_w - ext.width()) * 0.5, 136.0);
                    let _ = cr.show_text(label);
                }
            }
            drop(cr);
            save_ppm(surf, "/tmp/umi-mascot-expressions.ppm");
        }

        // 2. Full UMI Notch Showcase Grid (6 key states across 2 columns on clean dark wallpaper cards)
        {
            let gw = 1320_i32;
            let gh = 920_i32;
            let surf = cairo::ImageSurface::create(cairo::Format::ARgb32, gw, gh).expect("grid surf");
            let cr = Context::new(&surf).expect("cr");
            let bg = LinearGradient::new(0.0, 0.0, 0.0, gh as f64);
            bg.add_color_stop_rgb(0.0, 0.027, 0.035, 0.071);
            bg.add_color_stop_rgb(1.0, 0.047, 0.063, 0.118);
            let _ = cr.set_source(&bg);
            let _ = cr.paint();

            // Header bar
            cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
            cr.set_font_size(24.0);
            cr.set_source_rgb(0.93, 0.95, 0.98);
            cr.move_to(36.0, 48.0);
            let _ = cr.show_text("UMI Dynamic Island");
            cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
            cr.set_font_size(13.5);
            cr.set_source_rgba(0.49, 0.83, 0.99, 0.9);
            cr.move_to(36.0, 70.0);
            let _ = cr.show_text("Pure Rust GTK4 Layer-Shell Overlay · Fedora KDE Plasma Wayland");

            let panels: [(&str, NotchState, f64); 6] = [
                ("06 · Music & MPRIS2", NotchState::Music, 1.0),
                ("07 · Focus Pomodoro", NotchState::FocusTimer, 1.0),
                ("09 · Prayer Sun Arc", NotchState::Prayer, 1.0),
                ("10 · Water Reminder", NotchState::Water, 1.0),
                ("15 · Ask UMI AI", NotchState::AiPrompt, 1.0),
                ("17 · AI Limits (Codex Live)", NotchState::AiLimits, 1.0),
            ];

            let col_w = 610.0;
            let row_h = 248.0;
            for (idx, (title, st, mix)) in panels.iter().enumerate() {
                let col = (idx % 2) as f64;
                let row = (idx / 2) as f64;
                let px = 36.0 + col * (col_w + 28.0);
                let py = 96.0 + row * (row_h + 20.0);

                // Wallpaper card
                draw_rounded_rect(&cr, px, py, col_w, row_h, 16.0);
                let wbg = LinearGradient::new(px, py, px, py + row_h);
                wbg.add_color_stop_rgb(0.0, 0.74, 0.83, 0.97);
                wbg.add_color_stop_rgb(0.55, 0.45, 0.57, 0.82);
                wbg.add_color_stop_rgb(1.0, 0.16, 0.22, 0.36);
                let _ = cr.set_source(&wbg);
                let _ = cr.fill();

                let mut sm = StateMachine::new(cfg.pages.active.clone(), true);
                match st {
                    NotchState::Music => sm.jump_to_page(NotchPageId::Music),
                    NotchState::FocusTimer => sm.jump_to_page(NotchPageId::Focus),
                    NotchState::Prayer => sm.jump_to_page(NotchPageId::Prayer),
                    NotchState::Water => sm.jump_to_page(NotchPageId::Water),
                    NotchState::AiPrompt => sm.jump_to_page(NotchPageId::AskAi),
                    NotchState::AiLimits => sm.jump_to_page(NotchPageId::AiLimits),
                    _ => sm.current = st.clone(),
                }
                let geom = sm.current.tier(false).geometry();
                let mut spring = NotchSpring::new(geom.width_dp, geom.height_dp, geom.bottom_radius_dp, geom.ear_radius_dp);
                spring.expanded_mix.value = *mix;

                let sub = cairo::ImageSurface::create(cairo::Format::ARgb32, col_w as i32, 210).expect("sub");
                let sub_cr = Context::new(&sub).expect("sub_cr");
                let frame = RenderFrame::from_state(col_w, 210.0, &spring, 1.24, 1.2, false, false);
                draw_notch(&sub_cr, &frame, &sm, &cfg, &live);

                let _ = cr.save();
                draw_rounded_rect(&cr, px, py, col_w, row_h, 16.0);
                cr.clip();
                let _ = cr.set_source_surface(&sub, px, py);
                let _ = cr.paint();
                let _ = cr.restore();

                cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
                cr.set_font_size(12.5);
                cr.set_source_rgba(0.95, 0.97, 1.0, 0.92);
                cr.move_to(px + 18.0, py + row_h - 14.0);
                let _ = cr.show_text(title);
            }
            drop(cr);
            save_ppm(surf, "/tmp/umi-readme-showcase.ppm");
        }
    }
}
