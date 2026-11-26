use crate::config::UmiConfig;
use crate::domain::pomodoro::{PomodoroEngine, SegmentKind};
use crate::domain::prayer_calc::PrayerSchedule;
use crate::storage::{ShelfItem, TaskItem, WaterSummary};
use crate::ui::notch::renderer::RectF;
use chrono::{Datelike, Duration, Local};
use gtk4::cairo::{self, Context, LinearGradient};
use std::f64::consts::PI;

pub struct Phase3Data<'a> {
    pub pomodoro: &'a PomodoroEngine,
    pub prayer: &'a PrayerSchedule,
    pub water: &'a WaterSummary,
    pub shelf: &'a [ShelfItem],
    pub shelf_drag_over: bool,
    pub tasks: &'a [TaskItem],
    pub todo_edit_mode: bool,
    pub todo_scroll: usize,
    pub battery_pct: u8,
    pub battery_charging: bool,
}

pub struct FocusHitZones {
    pub pause_btn: RectF,
    pub reset_btn: RectF,
}

pub struct WaterHitZones {
    pub reminder_switch: RectF,
    pub drink_btn: RectF,
    pub settings_btn: RectF,
}

pub struct WaterSettingsHitZones {
    pub back_btn: RectF,
    pub goal_minus: RectF,
    pub goal_plus: RectF,
    pub bottle_minus: RectF,
    pub bottle_plus: RectF,
    pub int_45: RectF,
    pub int_60: RectF,
    pub int_90: RectF,
    pub open_full_btn: RectF,
}

pub struct ShelfHitZones {
    pub clear_btn: RectF,
    pub item_rects: Vec<RectF>,
}

pub struct TodoHitZones {
    pub add_btn: RectF,
    pub edit_btn: RectF,
    pub rows: [RectF; 3],
}

pub fn focus_hit_zones(x: f64, top_y: f64, unit: f64) -> FocusHitZones {
    FocusHitZones {
        pause_btn: RectF {
            x: x + 362.0 * unit,
            y: top_y + 34.0 * unit,
            w: 62.0 * unit,
            h: 22.0 * unit,
        },
        reset_btn: RectF {
            x: x + 432.0 * unit,
            y: top_y + 34.0 * unit,
            w: 22.0 * unit,
            h: 22.0 * unit,
        },
    }
}

pub fn water_hit_zones(x: f64, top_y: f64, unit: f64) -> WaterHitZones {
    WaterHitZones {
        reminder_switch: RectF {
            x: x + 184.0 * unit,
            y: top_y + 48.0 * unit,
            w: 36.0 * unit,
            h: 20.0 * unit,
        },
        drink_btn: RectF {
            x: x + 62.0 * unit,
            y: top_y + 120.0 * unit,
            w: 72.0 * unit,
            h: 26.0 * unit,
        },
        settings_btn: RectF {
            x: x + 142.0 * unit,
            y: top_y + 120.0 * unit,
            w: 84.0 * unit,
            h: 26.0 * unit,
        },
    }
}

pub fn water_settings_hit_zones(x: f64, top_y: f64, unit: f64) -> WaterSettingsHitZones {
    WaterSettingsHitZones {
        back_btn: RectF {
            x: x + 20.0 * unit,
            y: top_y + 8.0 * unit,
            w: 140.0 * unit,
            h: 26.0 * unit,
        },
        goal_minus: RectF {
            x: x + 286.0 * unit,
            y: top_y + 48.0 * unit,
            w: 26.0 * unit,
            h: 24.0 * unit,
        },
        goal_plus: RectF {
            x: x + 404.0 * unit,
            y: top_y + 48.0 * unit,
            w: 26.0 * unit,
            h: 24.0 * unit,
        },
        bottle_minus: RectF {
            x: x + 286.0 * unit,
            y: top_y + 84.0 * unit,
            w: 26.0 * unit,
            h: 24.0 * unit,
        },
        bottle_plus: RectF {
            x: x + 404.0 * unit,
            y: top_y + 84.0 * unit,
            w: 26.0 * unit,
            h: 24.0 * unit,
        },
        int_45: RectF {
            x: x + 286.0 * unit,
            y: top_y + 120.0 * unit,
            w: 44.0 * unit,
            h: 24.0 * unit,
        },
        int_60: RectF {
            x: x + 336.0 * unit,
            y: top_y + 120.0 * unit,
            w: 44.0 * unit,
            h: 24.0 * unit,
        },
        int_90: RectF {
            x: x + 386.0 * unit,
            y: top_y + 120.0 * unit,
            w: 44.0 * unit,
            h: 24.0 * unit,
        },
        open_full_btn: RectF {
            x: x + 356.0 * unit,
            y: top_y + 150.0 * unit,
            w: 150.0 * unit,
            h: 22.0 * unit,
        },
    }
}

pub fn shelf_hit_zones(x: f64, top_y: f64, unit: f64, count: usize) -> ShelfHitZones {
    let mut item_rects = Vec::new();
    for i in 0..count.min(4) {
        item_rects.push(RectF {
            x: x + (68.0 + i as f64 * 96.0) * unit,
            y: top_y + 44.0 * unit,
            w: 84.0 * unit,
            h: 96.0 * unit,
        });
    }
    ShelfHitZones {
        clear_btn: RectF {
            x: x + 84.0 * unit,
            y: top_y + 10.0 * unit,
            w: 46.0 * unit,
            h: 20.0 * unit,
        },
        item_rects,
    }
}

pub fn todo_hit_zones(x: f64, top_y: f64, unit: f64) -> TodoHitZones {
    TodoHitZones {
        add_btn: RectF {
            x: x + 56.0 * unit,
            y: top_y + 112.0 * unit,
            w: 84.0 * unit,
            h: 28.0 * unit,
        },
        edit_btn: RectF {
            x: x + 146.0 * unit,
            y: top_y + 112.0 * unit,
            w: 82.0 * unit,
            h: 28.0 * unit,
        },
        rows: [
            RectF { x: x + 244.0 * unit, y: top_y + 42.0 * unit, w: 224.0 * unit, h: 30.0 * unit },
            RectF { x: x + 244.0 * unit, y: top_y + 78.0 * unit, w: 224.0 * unit, h: 30.0 * unit },
            RectF { x: x + 244.0 * unit, y: top_y + 114.0 * unit, w: 224.0 * unit, h: 30.0 * unit },
        ],
    }
}

fn rrect(cr: &Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    let r = r.min(w * 0.5).min(h * 0.5);
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -PI * 0.5, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, PI * 0.5);
    cr.arc(x + r, y + h - r, r, PI * 0.5, PI);
    cr.arc(x + r, y + r, r, PI, PI * 1.5);
    cr.close_path();
}

fn trunc(s: &str, max_chars: usize) -> String {
    let mut ch = s.chars();
    let head: String = ch.by_ref().take(max_chars).collect();
    if ch.next().is_some() {
        format!("{}…", head.trim_end())
    } else {
        head
    }
}

/// State 07 · Focus Timer — 1:1 FigCraft Card 07 (546x182 @ 1.3x).
pub fn draw_state_07_focus(cr: &Context, x: f64, top_y: f64, unit: f64, p3: &Phase3Data<'_>, alpha: f64) {
    let pomo = p3.pomodoro;
    let is_break = pomo.segment_kind(pomo.segment_idx) != SegmentKind::Focus;

    // Left pane: session label, big countdown, finish estimate, progress bar
    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(11.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.45 * alpha);
    cr.move_to(x + 64.0 * unit, top_y + 47.0 * unit);
    let _ = cr.show_text(&pomo.label());

    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(30.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
    cr.move_to(x + 64.0 * unit, top_y + 82.0 * unit);
    let _ = cr.show_text(&pomo.remaining_mm_ss());

    let end_str = pomo.estimated_end_hh_mm();
    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(13.5 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
    cr.move_to(x + 64.0 * unit, top_y + 106.0 * unit);
    let _ = cr.show_text(&end_str);

    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(12.0 * unit);
    if is_break {
        cr.set_source_rgba(0.133, 0.827, 0.933, alpha);
    } else {
        cr.set_source_rgba(0.659, 0.333, 0.965, alpha); // #A855F6
    }
    cr.move_to(x + 112.0 * unit, top_y + 106.0 * unit);
    let _ = cr.show_text("until done");

    // Progress bar (64, 124, 142x4)
    rrect(cr, x + 64.0 * unit, top_y + 124.0 * unit, 142.0 * unit, 4.0 * unit, 2.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.14 * alpha);
    let _ = cr.fill();

    let fill_w = (142.0 * pomo.segment_progress()).max(6.0);
    let bar_g = LinearGradient::new(x + 64.0 * unit, 0.0, x + (64.0 + fill_w) * unit, 0.0);
    if is_break {
        bar_g.add_color_stop_rgba(0.0, 0.133, 0.827, 0.933, alpha);
        bar_g.add_color_stop_rgba(1.0, 0.204, 0.827, 0.600, alpha);
    } else {
        bar_g.add_color_stop_rgba(0.0, 0.357, 0.424, 1.0, alpha);
        bar_g.add_color_stop_rgba(1.0, 0.659, 0.333, 0.965, alpha);
    }
    rrect(cr, x + 64.0 * unit, top_y + 124.0 * unit, fill_w * unit, 4.0 * unit, 2.0 * unit);
    let _ = cr.set_source(&bar_g);
    let _ = cr.fill();

    // Vertical divider (226, 40, 1x96)
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.12 * alpha);
    cr.rectangle(x + 226.0 * unit, top_y + 40.0 * unit, 1.0 * unit, 96.0 * unit);
    let _ = cr.fill();

    // Right pane: "This round", Pause/Start pill, Reset button
    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(11.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.45 * alpha);
    cr.move_to(x + 246.0 * unit, top_y + 47.0 * unit);
    let _ = cr.show_text("This round");

    rrect(cr, x + 362.0 * unit, top_y + 34.0 * unit, 62.0 * unit, 22.0 * unit, 11.0 * unit);
    if pomo.running {
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.14 * alpha);
    } else {
        cr.set_source_rgba(0.357, 0.424, 1.0, 0.55 * alpha);
    }
    let _ = cr.fill();
    let btn_txt = if pomo.running { "Pause" } else { "Start" };
    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(11.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
    if let Ok(ext) = cr.text_extents(btn_txt) {
        cr.move_to(x + (362.0 + 31.0) * unit - ext.width() * 0.5, top_y + 49.0 * unit);
        let _ = cr.show_text(btn_txt);
    }

    // Reset circular button (432, 34, 22x22)
    let rcx = x + 443.0 * unit;
    let rcy = top_y + 45.0 * unit;
    cr.new_sub_path();
    cr.arc(rcx, rcy, 11.0 * unit, 0.0, 2.0 * PI);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.10 * alpha);
    let _ = cr.fill();
    cr.new_sub_path();
    cr.arc(rcx, rcy, 4.8 * unit, -PI * 0.35, PI * 1.25);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.85 * alpha);
    cr.set_line_width(1.5 * unit);
    cr.set_line_cap(cairo::LineCap::Round);
    let _ = cr.stroke();

    // 8-segment timeline matching FigCraft Nodes 286..295
    let seg_specs: [(f64, f64, f64, f64, bool); 8] = [
        (246.0, 78.0, 32.0, 6.0, true),
        (282.0, 79.0, 10.0, 4.0, false),
        (296.0, 78.0, 32.0, 6.0, true),
        (332.0, 79.0, 10.0, 4.0, false),
        (346.0, 78.0, 32.0, 6.0, true),
        (382.0, 79.0, 10.0, 4.0, false),
        (396.0, 78.0, 32.0, 6.0, true),
        (432.0, 79.0, 36.0, 4.0, false),
    ];

    for (idx, &(sx, sy, sw, sh, is_focus)) in seg_specs.iter().enumerate() {
        rrect(cr, x + sx * unit, top_y + sy * unit, sw * unit, sh * unit, sh * 0.5 * unit);
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.13 * alpha);
        let _ = cr.fill();

        let fill_frac = if idx < pomo.segment_idx {
            1.0
        } else if idx == pomo.segment_idx {
            pomo.segment_progress()
        } else {
            0.0
        };

        if fill_frac > 0.01 {
            let fw = (sw * fill_frac).max(sh);
            let sg = LinearGradient::new(x + sx * unit, 0.0, x + (sx + sw) * unit, 0.0);
            if is_focus {
                sg.add_color_stop_rgba(0.0, 0.357, 0.424, 1.0, alpha);
                sg.add_color_stop_rgba(1.0, 0.659, 0.333, 0.965, alpha);
            } else {
                sg.add_color_stop_rgba(0.0, 0.133, 0.827, 0.933, alpha);
                sg.add_color_stop_rgba(1.0, 0.204, 0.827, 0.600, alpha);
            }
            rrect(cr, x + sx * unit, top_y + sy * unit, fw * unit, sh * unit, sh * 0.5 * unit);
            let _ = cr.set_source(&sg);
            let _ = cr.fill();
        }

        // Glowing head on active segment
        if idx == pomo.segment_idx {
            let hx = x + (sx + sw * fill_frac) * unit;
            let hy = top_y + (sy + sh * 0.5) * unit;
            cr.new_sub_path();
            cr.arc(hx, hy, 8.0 * unit, 0.0, 2.0 * PI);
            cr.set_source_rgba(0.659, 0.333, 0.965, 0.32 * alpha);
            let _ = cr.fill();
            cr.new_sub_path();
            cr.arc(hx, hy, 4.8 * unit, 0.0, 2.0 * PI);
            cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
            let _ = cr.fill();
        }
    }

    // Segment labels: 1 2 3 4 Long
    let labels = [(259.0, "1", 0), (309.0, "2", 2), (359.0, "3", 4), (409.0, "4", 6), (439.0, "Long", 7)];
    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(9.5 * unit);
    for (lx, txt, seg_id) in labels {
        let active = pomo.segment_idx == seg_id || (seg_id < 7 && pomo.segment_idx == seg_id + 1);
        cr.set_source_rgba(1.0, 1.0, 1.0, (if active { 1.0 } else { 0.50 }) * alpha);
        cr.move_to(x + lx * unit, top_y + 106.0 * unit);
        let _ = cr.show_text(txt);
    }

    cr.set_font_size(10.5 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.45 * alpha);
    cr.move_to(x + 246.0 * unit, top_y + 132.0 * unit);
    let _ = cr.show_text(&pomo.round_summary());
}

/// State 09 · Prayer — 1:1 FigCraft Card 09 (546x182 @ 1.3x, offline Sun Arc).
pub fn draw_state_09_prayer(cr: &Context, x: f64, top_y: f64, unit: f64, p3: &Phase3Data<'_>, alpha: f64) {
    let sched = p3.prayer;
    let next = &sched.entries[sched.next_idx];

    // City subtitle next to "Prayer" title in header
    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(12.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.45 * alpha);
    cr.move_to(x + 82.0 * unit, top_y + 23.0 * unit);
    let _ = cr.show_text(&sched.city);

    // Left pane: Next / Ashar / 15:18 in 42 min / progress bar
    cr.set_font_size(11.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.45 * alpha);
    cr.move_to(x + 64.0 * unit, top_y + 45.0 * unit);
    let _ = cr.show_text("Next");

    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(27.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
    cr.move_to(x + 64.0 * unit, top_y + 79.0 * unit);
    let _ = cr.show_text(next.name);

    cr.set_font_size(13.5 * unit);
    cr.move_to(x + 64.0 * unit, top_y + 106.0 * unit);
    let _ = cr.show_text(&next.hh_mm());

    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(12.0 * unit);
    cr.set_source_rgba(0.392, 0.710, 1.0, alpha); // #64B5FF
    cr.move_to(x + 110.0 * unit, top_y + 106.0 * unit);
    let _ = cr.show_text(&sched.countdown_text);

    rrect(cr, x + 64.0 * unit, top_y + 124.0 * unit, 142.0 * unit, 4.0 * unit, 2.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.14 * alpha);
    let _ = cr.fill();
    let fw = (142.0 * sched.segment_progress).max(8.0);
    let pg = LinearGradient::new(x + 64.0 * unit, 0.0, x + 206.0 * unit, 0.0);
    pg.add_color_stop_rgba(0.0, 0.098, 0.722, 1.0, alpha);
    pg.add_color_stop_rgba(0.55, 0.176, 0.357, 1.0, alpha);
    pg.add_color_stop_rgba(1.0, 0.478, 0.294, 1.0, alpha);
    rrect(cr, x + 64.0 * unit, top_y + 124.0 * unit, fw * unit, 4.0 * unit, 2.0 * unit);
    let _ = cr.set_source(&pg);
    let _ = cr.fill();

    // Divider (226, 40, 1x96)
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.12 * alpha);
    cr.rectangle(x + 226.0 * unit, top_y + 40.0 * unit, 1.0 * unit, 96.0 * unit);
    let _ = cr.fill();

    // Right pane: Horizon line + Sun Arc curve (x = 248..476, horizon y = 106, peak y = 62)
    let ax0 = x + 248.0 * unit;
    let aw = 228.0 * unit;
    let hy = top_y + 106.0 * unit;
    let amp = 44.0 * unit;

    cr.set_source_rgba(1.0, 1.0, 1.0, 0.14 * alpha);
    cr.rectangle(ax0, hy, aw, 1.0 * unit);
    let _ = cr.fill();

    // Full dim arc
    cr.new_sub_path();
    for step in 0..=40 {
        let t = step as f64 / 40.0;
        let px = ax0 + t * aw;
        let py = hy - (t * PI).sin() * amp;
        if step == 0 {
            cr.move_to(px, py);
        } else {
            cr.line_to(px, py);
        }
    }
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.20 * alpha);
    cr.set_line_width(2.0 * unit);
    cr.set_line_cap(cairo::LineCap::Round);
    let _ = cr.stroke();

    // Active illuminated arc up to sun_arc_progress
    let p_cur = sched.sun_arc_progress.clamp(0.02, 0.98);
    let ag = LinearGradient::new(ax0, hy, ax0 + aw, hy);
    ag.add_color_stop_rgba(0.0, 0.098, 0.722, 1.0, alpha);
    ag.add_color_stop_rgba(0.6, 0.357, 0.424, 1.0, alpha);
    ag.add_color_stop_rgba(1.0, 0.961, 0.620, 0.043, alpha);
    cr.new_sub_path();
    for step in 0..=32 {
        let t = (step as f64 / 32.0) * p_cur;
        let px = ax0 + t * aw;
        let py = hy - (t * PI).sin() * amp;
        if step == 0 {
            cr.move_to(px, py);
        } else {
            cr.line_to(px, py);
        }
    }
    let _ = cr.set_source(&ag);
    cr.set_line_width(2.4 * unit);
    let _ = cr.stroke();

    // Glowing sun marker
    let sx = ax0 + p_cur * aw;
    let sy = hy - (p_cur * PI).sin() * amp;
    cr.new_sub_path();
    cr.arc(sx, sy, 8.5 * unit, 0.0, 2.0 * PI);
    cr.set_source_rgba(0.220, 0.741, 0.973, 0.32 * alpha);
    let _ = cr.fill();
    cr.new_sub_path();
    cr.arc(sx, sy, 3.8 * unit, 0.0, 2.0 * PI);
    cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
    let _ = cr.fill();

    // 5 prayer labels + times underneath (x = 242, 298, 354, 404, 456)
    let xs = [242.0, 296.0, 352.0, 400.0, 454.0];
    for (idx, entry) in sched.entries.iter().enumerate() {
        let is_next = idx == sched.next_idx;
        let a = (if is_next { 1.0 } else { 0.45 }) * alpha;
        cr.select_font_face(
            "Inter",
            cairo::FontSlant::Normal,
            if is_next { cairo::FontWeight::Bold } else { cairo::FontWeight::Normal },
        );
        cr.set_font_size(8.8 * unit);
        cr.set_source_rgba(1.0, 1.0, 1.0, a);
        cr.move_to(x + xs[idx] * unit, top_y + 124.0 * unit);
        let _ = cr.show_text(entry.name);
        cr.move_to(x + xs[idx] * unit, top_y + 136.0 * unit);
        let _ = cr.show_text(&entry.hh_mm());
    }
}

/// State 10 · Water — 1:1 FigCraft Card 10 (546x182 @ 1.3x).
pub fn draw_state_10_water(
    cr: &Context,
    x: f64,
    top_y: f64,
    unit: f64,
    cfg: &UmiConfig,
    p3: &Phase3Data<'_>,
    alpha: f64,
) {
    let goal_ml = cfg.water.daily_goal_ml.max(250);
    let today_ml = p3.water.today_ml;
    let pct = ((today_ml as f64 / goal_ml as f64) * 100.0).round() as u32;
    let ratio = (today_ml as f64 / goal_ml as f64).clamp(0.0, 1.0);

    let liters = format!("{:.1}", today_ml as f64 / 1000.0);
    let goal_str = format!("/ {:.1} L", goal_ml as f64 / 1000.0);

    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(29.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
    cr.move_to(x + 62.0 * unit, top_y + 70.0 * unit);
    let _ = cr.show_text(&liters);

    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(13.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.45 * alpha);
    cr.move_to(x + 110.0 * unit, top_y + 68.0 * unit);
    let _ = cr.show_text(&goal_str);

    // Toggle switch (184, 48, 36x20)
    rrect(cr, x + 184.0 * unit, top_y + 48.0 * unit, 36.0 * unit, 20.0 * unit, 10.0 * unit);
    if cfg.water.reminder_enabled {
        cr.set_source_rgba(0.149, 0.600, 1.0, alpha); // #2699FF
    } else {
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.16 * alpha);
    }
    let _ = cr.fill();
    let knob_x = if cfg.water.reminder_enabled { 210.0 } else { 194.0 };
    cr.new_sub_path();
    cr.arc(x + knob_x * unit, top_y + 58.0 * unit, 8.0 * unit, 0.0, 2.0 * PI);
    cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
    let _ = cr.fill();

    // Status line: Today · 100% · Goal reached
    cr.set_font_size(10.8 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.45 * alpha);
    cr.move_to(x + 62.0 * unit, top_y + 91.0 * unit);
    let _ = cr.show_text("Today ·");
    cr.set_source_rgba(0.310, 0.753, 1.0, alpha);
    cr.move_to(x + 104.0 * unit, top_y + 91.0 * unit);
    let _ = cr.show_text(&format!("{pct}%"));
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.45 * alpha);
    cr.move_to(x + 138.0 * unit, top_y + 91.0 * unit);
    let suffix = if today_ml >= goal_ml {
        "· Goal reached".to_string()
    } else {
        format!("· {:.1} L left", (goal_ml - today_ml) as f64 / 1000.0)
    };
    let _ = cr.show_text(&suffix);

    // Blue progress bar (62, 102, 154x5)
    rrect(cr, x + 62.0 * unit, top_y + 102.0 * unit, 154.0 * unit, 5.0 * unit, 2.5 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.12 * alpha);
    let _ = cr.fill();
    if ratio > 0.01 {
        let wg = LinearGradient::new(x + 62.0 * unit, 0.0, x + 216.0 * unit, 0.0);
        wg.add_color_stop_rgba(0.0, 0.400, 0.859, 1.0, alpha);
        wg.add_color_stop_rgba(1.0, 0.176, 0.420, 1.0, alpha);
        rrect(cr, x + 62.0 * unit, top_y + 102.0 * unit, (154.0 * ratio).max(5.0) * unit, 5.0 * unit, 2.5 * unit);
        let _ = cr.set_source(&wg);
        let _ = cr.fill();
    }

    // +Drink button (62, 120, 72x26, #0D4A86)
    rrect(cr, x + 62.0 * unit, top_y + 120.0 * unit, 72.0 * unit, 26.0 * unit, 13.0 * unit);
    cr.set_source_rgba(0.051, 0.290, 0.525, alpha);
    let _ = cr.fill();
    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(11.5 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
    cr.move_to(x + 76.0 * unit, top_y + 137.0 * unit);
    let _ = cr.show_text("+Drink");

    // Settings button (142, 120, 84x26)
    rrect(cr, x + 142.0 * unit, top_y + 120.0 * unit, 84.0 * unit, 26.0 * unit, 13.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.12 * alpha);
    let _ = cr.fill();
    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(11.5 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.92 * alpha);
    cr.move_to(x + 160.0 * unit, top_y + 137.0 * unit);
    let _ = cr.show_text("Settings");

    // Divider (240, 40, 1x104)
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.12 * alpha);
    cr.rectangle(x + 240.0 * unit, top_y + 40.0 * unit, 1.0 * unit, 104.0 * unit);
    let _ = cr.fill();

    // 8 hourly bar tracks (x = 256 + i*18, y = 68, 8x58)
    for i in 0..8 {
        let bx = x + (256.0 + i as f64 * 18.0) * unit;
        let by = top_y + 68.0 * unit;
        let bh = 58.0 * unit;
        rrect(cr, bx, by, 8.0 * unit, bh, 4.0 * unit);
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.07 * alpha);
        let _ = cr.fill();

        let ml = p3.water.hourly_ml[i];
        if ml > 0 {
            let frac = (ml as f64 / 400.0).clamp(0.10, 1.0);
            let fh = bh * frac;
            let fy = by + bh - fh;
            let bg = LinearGradient::new(bx, fy, bx, by + bh);
            bg.add_color_stop_rgba(0.0, 0.361, 0.776, 1.0, alpha);
            bg.add_color_stop_rgba(1.0, 0.110, 0.388, 0.878, alpha);
            rrect(cr, bx, fy, 8.0 * unit, fh, 4.0 * unit);
            let _ = cr.set_source(&bg);
            let _ = cr.fill();
        }
    }

    // Axis labels
    cr.set_font_size(9.5 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.45 * alpha);
    cr.move_to(x + 412.0 * unit, top_y + 73.0 * unit);
    let _ = cr.show_text("400");
    cr.move_to(x + 412.0 * unit, top_y + 100.0 * unit);
    let _ = cr.show_text("200");
    cr.move_to(x + 412.0 * unit, top_y + 127.0 * unit);
    let _ = cr.show_text("0 ml");

    cr.move_to(x + 254.0 * unit, top_y + 144.0 * unit);
    let _ = cr.show_text("09");
    cr.move_to(x + 308.0 * unit, top_y + 144.0 * unit);
    let _ = cr.show_text("12");
    cr.move_to(x + 362.0 * unit, top_y + 144.0 * unit);
    let _ = cr.show_text("15");
}

/// State 10b · In-Notch Water Settings — matches design.figcraft Page Settings Feature.
pub fn draw_in_notch_water_settings(cr: &Context, x: f64, top_y: f64, unit: f64, cfg: &UmiConfig, alpha: f64) {
    // Back header "< Water Settings"
    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(13.5 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
    cr.move_to(x + 24.0 * unit, top_y + 23.0 * unit);
    let _ = cr.show_text("<  Water ");
    let w1 = cr.text_extents("<  Water ").map(|e| e.x_advance()).unwrap_or(68.0 * unit);
    cr.set_source_rgba(0.333, 0.784, 1.0, alpha);
    cr.move_to(x + 24.0 * unit + w1, top_y + 23.0 * unit);
    let _ = cr.show_text("Settings");

    // Row 1: Daily Goal
    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(12.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.75 * alpha);
    cr.move_to(x + 76.0 * unit, top_y + 64.0 * unit);
    let _ = cr.show_text("Daily Goal");
    draw_stepper(cr, x + 286.0 * unit, top_y + 48.0 * unit, unit, &format!("{:.1} L", cfg.water.daily_goal_ml as f64 / 1000.0), alpha);

    // Row 2: Bottle Size
    cr.set_font_size(12.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.75 * alpha);
    cr.move_to(x + 76.0 * unit, top_y + 100.0 * unit);
    let _ = cr.show_text("Bottle Size");
    draw_stepper(cr, x + 286.0 * unit, top_y + 84.0 * unit, unit, &format!("{} ml", cfg.water.bottle_ml), alpha);

    // Row 3: Remind Every
    cr.set_font_size(12.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.75 * alpha);
    cr.move_to(x + 76.0 * unit, top_y + 136.0 * unit);
    let _ = cr.show_text("Remind Every");

    for (idx, mins) in [45u32, 60, 90].iter().enumerate() {
        let px = x + (286.0 + idx as f64 * 50.0) * unit;
        let py = top_y + 120.0 * unit;
        let active = cfg.water.reminder_interval_min == *mins;
        rrect(cr, px, py, 44.0 * unit, 24.0 * unit, 8.0 * unit);
        if active {
            cr.set_source_rgba(0.149, 0.600, 1.0, alpha);
        } else {
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.10 * alpha);
        }
        let _ = cr.fill();
        cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        cr.set_font_size(10.5 * unit);
        cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
        cr.move_to(px + 10.0 * unit, py + 15.5 * unit);
        let _ = cr.show_text(&format!("{mins}m"));
    }

    // Bottom link: "Open Full Settings →"
    rrect(cr, x + 356.0 * unit, top_y + 150.0 * unit, 150.0 * unit, 22.0 * unit, 8.0 * unit);
    cr.set_source_rgba(0.216, 0.741, 0.969, 0.14 * alpha);
    let _ = cr.fill();
    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(10.0 * unit);
    cr.set_source_rgba(0.490, 0.827, 0.988, alpha);
    cr.move_to(x + 372.0 * unit, top_y + 164.5 * unit);
    let _ = cr.show_text("Open Full Settings →");
}

fn draw_stepper(cr: &Context, sx: f64, sy: f64, unit: f64, value: &str, alpha: f64) {
    // Minus pill
    rrect(cr, sx, sy, 26.0 * unit, 24.0 * unit, 7.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.12 * alpha);
    let _ = cr.fill();
    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(14.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
    cr.move_to(sx + 9.5 * unit, sy + 16.5 * unit);
    let _ = cr.show_text("-");

    // Value centered
    cr.set_font_size(12.5 * unit);
    if let Ok(ext) = cr.text_extents(value) {
        cr.move_to(sx + 72.0 * unit - ext.width() * 0.5, sy + 16.0 * unit);
        let _ = cr.show_text(value);
    }

    // Plus pill
    rrect(cr, sx + 118.0 * unit, sy, 26.0 * unit, 24.0 * unit, 7.0 * unit);
    cr.set_source_rgba(0.149, 0.600, 1.0, 0.45 * alpha);
    let _ = cr.fill();
    cr.set_font_size(14.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
    cr.move_to(sx + 126.5 * unit, sy + 16.5 * unit);
    let _ = cr.show_text("+");
}

/// State 11 · Shelf — 1:1 FigCraft Card 11 (546x182 @ 1.3x, Wayland DnD).
pub fn draw_state_11_shelf(cr: &Context, x: f64, top_y: f64, unit: f64, p3: &Phase3Data<'_>, alpha: f64) {
    // Clear pill next to "Shelf" title (84, 11, 44x18)
    rrect(cr, x + 84.0 * unit, top_y + 11.0 * unit, 44.0 * unit, 18.0 * unit, 9.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.10 * alpha);
    let _ = cr.fill();
    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(10.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.60 * alpha);
    cr.move_to(x + 93.0 * unit, top_y + 23.0 * unit);
    let _ = cr.show_text("Clear");

    // Dashed Drop Zone (56, 38, 434x112)
    rrect(cr, x + 56.0 * unit, top_y + 38.0 * unit, 434.0 * unit, 112.0 * unit, 14.0 * unit);
    if p3.shelf_drag_over {
        cr.set_source_rgba(0.220, 0.741, 0.973, 0.14 * alpha);
    } else {
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.025 * alpha);
    }
    let _ = cr.fill();

    let _ = cr.save();
    rrect(cr, x + 56.0 * unit, top_y + 38.0 * unit, 434.0 * unit, 112.0 * unit, 14.0 * unit);
    cr.set_dash(&[5.0 * unit, 4.0 * unit], 0.0);
    cr.set_line_width(1.2 * unit);
    if p3.shelf_drag_over {
        cr.set_source_rgba(0.220, 0.741, 0.973, 0.85 * alpha);
    } else {
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.16 * alpha);
    }
    let _ = cr.stroke();
    let _ = cr.restore();

    // Render stored shelf items (up to 4 tiles)
    for (idx, item) in p3.shelf.iter().take(4).enumerate() {
        let tx = x + (74.0 + idx as f64 * 96.0) * unit;
        let ty = top_y + 48.0 * unit;
        draw_file_thumb(cr, tx, ty, 48.0 * unit, &item.name, alpha);

        cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        cr.set_font_size(10.5 * unit);
        cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
        cr.move_to(tx - 4.0 * unit, top_y + 112.0 * unit);
        let _ = cr.show_text(&trunc(&item.name, 12));

        cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
        cr.set_font_size(9.5 * unit);
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.45 * alpha);
        cr.move_to(tx + 4.0 * unit, top_y + 126.0 * unit);
        let _ = cr.show_text(&format_bytes(item.size_bytes));
    }

    // Drop hint centered in remaining space
    if p3.shelf.len() < 3 {
        let hint_x = if p3.shelf.is_empty() {
            x + 215.0 * unit
        } else {
            x + (224.0 + (p3.shelf.len().saturating_sub(1) as f64) * 55.0) * unit
        };
        cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        cr.set_font_size(13.0 * unit);
        cr.set_source_rgba(1.0, 1.0, 1.0, (if p3.shelf_drag_over { 0.95 } else { 0.55 }) * alpha);
        cr.move_to(hint_x, top_y + 92.0 * unit);
        let _ = cr.show_text(if p3.shelf_drag_over { "Release to pin file" } else { "Drop files here" });

        cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
        cr.set_font_size(10.0 * unit);
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.35 * alpha);
        cr.move_to(hint_x - 28.0 * unit, top_y + 110.0 * unit);
        let _ = cr.show_text("Drag out anytime · click to open");
    }
}

fn draw_file_thumb(cr: &Context, x: f64, y: f64, sz: f64, name: &str, alpha: f64) {
    let ext = name
        .rsplit('.')
        .next()
        .unwrap_or("FILE")
        .to_ascii_uppercase();
    let _ = cr.save();
    rrect(cr, x, y, sz, sz, sz * 0.20);
    cr.clip();

    // Vibrant FigCraft sky-ocean-sand thumbnail look for images/design files, indigo/emerald for code/docs
    if matches!(ext.as_str(), "PNG" | "JPG" | "JPEG" | "WEBP" | "FIG" | "FIGCRAFT" | "SVG") {
        cr.set_source_rgba(0.216, 0.741, 0.969, alpha); // #37BDF7
        cr.rectangle(x, y, sz, sz);
        let _ = cr.fill();
        cr.new_sub_path();
        cr.arc(x + sz * 0.72, y + sz * 0.25, sz * 0.09, 0.0, 2.0 * PI);
        cr.set_source_rgba(0.988, 0.878, 0.278, alpha);
        let _ = cr.fill();
        cr.set_source_rgba(0.008, 0.518, 0.780, alpha);
        cr.rectangle(x, y + sz * 0.54, sz, sz * 0.28);
        let _ = cr.fill();
        cr.set_source_rgba(0.918, 0.702, 0.031, alpha);
        cr.rectangle(x, y + sz * 0.75, sz, sz * 0.25);
        let _ = cr.fill();
    } else {
        let g = LinearGradient::new(x, y, x + sz, y + sz);
        g.add_color_stop_rgba(0.0, 0.388, 0.400, 0.945, alpha);
        g.add_color_stop_rgba(1.0, 0.220, 0.741, 0.973, alpha);
        let _ = cr.set_source(&g);
        cr.rectangle(x, y, sz, sz);
        let _ = cr.fill();

        cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        cr.set_font_size(sz * 0.24);
        cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
        let tag = trunc(&ext, 4);
        if let Ok(e) = cr.text_extents(&tag) {
            cr.move_to(x + (sz - e.width()) * 0.5, y + sz * 0.60);
            let _ = cr.show_text(&tag);
        }
    }
    let _ = cr.restore();
}

/// State 13 · To-do — 1:1 FigCraft Card 13 (546x182 @ 1.3x).
pub fn draw_state_13_todo(cr: &Context, x: f64, top_y: f64, unit: f64, p3: &Phase3Data<'_>, alpha: f64) {
    let total = p3.tasks.len();
    let done = p3.tasks.iter().filter(|t| t.completed).count();

    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(32.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
    cr.move_to(x + 76.0 * unit, top_y + 72.0 * unit);
    let _ = cr.show_text(&format!("{done}/{total}"));

    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(11.5 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.45 * alpha);
    cr.move_to(x + 60.0 * unit, top_y + 94.0 * unit);
    let _ = cr.show_text("Task Done Today");

    // Add Task pill (56, 112, 84x28)
    rrect(cr, x + 56.0 * unit, top_y + 112.0 * unit, 84.0 * unit, 28.0 * unit, 14.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.12 * alpha);
    let _ = cr.fill();
    cr.new_sub_path();
    cr.arc(x + 70.0 * unit, top_y + 126.0 * unit, 6.0 * unit, 0.0, 2.0 * PI);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.30 * alpha);
    let _ = cr.fill();
    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(10.5 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
    cr.move_to(x + 66.8 * unit, top_y + 129.5 * unit);
    let _ = cr.show_text("+");
    cr.move_to(x + 81.0 * unit, top_y + 130.0 * unit);
    let _ = cr.show_text("Add Task");

    // Edit Task pill (146, 112, 82x28)
    rrect(cr, x + 146.0 * unit, top_y + 112.0 * unit, 82.0 * unit, 28.0 * unit, 14.0 * unit);
    if p3.todo_edit_mode {
        cr.set_source_rgba(0.937, 0.267, 0.267, 0.45 * alpha);
    } else {
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.12 * alpha);
    }
    let _ = cr.fill();
    cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
    cr.move_to(x + 162.0 * unit, top_y + 130.0 * unit);
    let _ = cr.show_text(if p3.todo_edit_mode { "Done Edit" } else { "Edit Task" });

    // Task rows (x=244, y=42/78/114, 224x30)
    let ys = [42.0_f64, 78.0, 114.0];
    if p3.tasks.is_empty() {
        rrect(cr, x + 244.0 * unit, top_y + 42.0 * unit, 224.0 * unit, 102.0 * unit, 12.0 * unit);
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.05 * alpha);
        let _ = cr.fill();
        cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
        cr.set_font_size(11.5 * unit);
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.45 * alpha);
        cr.move_to(x + 266.0 * unit, top_y + 96.0 * unit);
        let _ = cr.show_text("No tasks yet · click + Add Task");
    } else {
        for (slot, row_y) in ys.iter().enumerate() {
            let Some(task) = p3.tasks.get(p3.todo_scroll + slot) else {
                continue;
            };
            let ry = top_y + row_y * unit;
            rrect(cr, x + 244.0 * unit, ry, 224.0 * unit, 30.0 * unit, 9.0 * unit);
            if task.completed {
                cr.set_source_rgba(0.157, 0.471, 0.235, 0.38 * alpha); // #28783C@0.38
            } else {
                cr.set_source_rgba(1.0, 1.0, 1.0, 0.08 * alpha);
            }
            let _ = cr.fill();

            // Check circle or delete circle (252, y+7, 16x16)
            let ccx = x + 260.0 * unit;
            let ccy = ry + 15.0 * unit;
            cr.new_sub_path();
            cr.arc(ccx, ccy, 7.5 * unit, 0.0, 2.0 * PI);
            if p3.todo_edit_mode {
                cr.set_source_rgba(0.937, 0.267, 0.267, 0.90 * alpha);
                let _ = cr.fill();
                cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
                cr.set_line_width(1.6 * unit);
                cr.new_sub_path();
                cr.move_to(ccx - 2.8 * unit, ccy - 2.8 * unit);
                cr.line_to(ccx + 2.8 * unit, ccy + 2.8 * unit);
                cr.move_to(ccx + 2.8 * unit, ccy - 2.8 * unit);
                cr.line_to(ccx - 2.8 * unit, ccy + 2.8 * unit);
                let _ = cr.stroke();
            } else if task.completed {
                cr.set_source_rgba(0.184, 0.816, 0.420, alpha); // #2FD06B
                let _ = cr.fill();
                cr.new_sub_path();
                cr.move_to(ccx - 3.2 * unit, ccy + 0.2 * unit);
                cr.line_to(ccx - 0.8 * unit, ccy + 2.8 * unit);
                cr.line_to(ccx + 3.6 * unit, ccy - 2.4 * unit);
                cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
                cr.set_line_width(1.7 * unit);
                cr.set_line_cap(cairo::LineCap::Round);
                cr.set_line_join(cairo::LineJoin::Round);
                let _ = cr.stroke();
            } else {
                cr.set_source_rgba(1.0, 1.0, 1.0, 0.35 * alpha);
                cr.set_line_width(1.4 * unit);
                let _ = cr.stroke();
            }

            let label = trunc(&task.title, 26);
            cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
            cr.set_font_size(11.8 * unit);
            cr.set_source_rgba(1.0, 1.0, 1.0, (if task.completed { 0.55 } else { 0.95 }) * alpha);
            cr.move_to(x + 276.0 * unit, ry + 19.0 * unit);
            let _ = cr.show_text(&label);

            if task.completed {
                let tw = cr.text_extents(&label).map(|e| e.width()).unwrap_or(90.0 * unit);
                cr.set_source_rgba(1.0, 1.0, 1.0, 0.40 * alpha);
                cr.rectangle(x + 276.0 * unit, ry + 15.0 * unit, tw, 1.0 * unit);
                let _ = cr.fill();
            }
        }

        // Scrollbar thumb (478, 44, 3x42)
        if p3.tasks.len() > 3 {
            let max_scroll = (p3.tasks.len() - 3) as f64;
            let frac = (p3.todo_scroll as f64 / max_scroll).clamp(0.0, 1.0);
            rrect(cr, x + 478.0 * unit, top_y + (44.0 + frac * 54.0) * unit, 3.0 * unit, 42.0 * unit, 1.5 * unit);
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.40 * alpha);
            let _ = cr.fill();
        }
    }
}

/// State 14 · Widgets — 1:1 FigCraft Card 14 (546x182 @ 1.3x: THIS WEEK + BATTERY).
pub fn draw_state_14_widgets(cr: &Context, x: f64, top_y: f64, unit: f64, p3: &Phase3Data<'_>, alpha: f64) {
    // Left Widget: THIS WEEK (58, 40, 204x112)
    rrect(cr, x + 58.0 * unit, top_y + 40.0 * unit, 204.0 * unit, 112.0 * unit, 14.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.06 * alpha);
    let _ = cr.fill();

    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(9.5 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.45 * alpha);
    cr.move_to(x + 70.0 * unit, top_y + 56.0 * unit);
    let _ = cr.show_text("THIS WEEK");

    let today = Local::now().date_naive();
    let weekday_idx = today.weekday().num_days_from_monday() as i64;
    let monday = today - Duration::days(weekday_idx);
    let day_letters = ["M", "T", "W", "T", "F", "S", "S"];

    for i in 0..7i64 {
        let d = monday + Duration::days(i);
        let col_x = x + (77.0 + i as f64 * 25.5) * unit;
        let is_today = i == weekday_idx;

        cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
        cr.set_font_size(9.0 * unit);
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.45 * alpha);
        cr.move_to(col_x - 3.5 * unit, top_y + 72.0 * unit);
        let _ = cr.show_text(day_letters[i as usize]);

        if is_today {
            cr.new_sub_path();
            cr.arc(col_x, top_y + 85.0 * unit, 9.0 * unit, 0.0, 2.0 * PI);
            cr.set_source_rgba(0.039, 0.518, 1.0, alpha); // #0A84FF
            let _ = cr.fill();
        }

        let num_s = format!("{}", d.day());
        cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        cr.set_font_size(10.5 * unit);
        cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
        if let Ok(ext) = cr.text_extents(&num_s) {
            cr.move_to(col_x - ext.width() * 0.5, top_y + 89.0 * unit);
            let _ = cr.show_text(&num_s);
        }
    }

    // Event dots row (y=100)
    let dot_cols = [(0, (0.039, 0.518, 1.0)), (weekday_idx as usize, (0.188, 0.780, 0.349)), (4, (1.0, 0.271, 0.224)), (6, (1.0, 0.624, 0.039))];
    for (day_i, rgb) in dot_cols {
        let col_x = x + (77.0 + day_i as f64 * 25.5) * unit;
        cr.new_sub_path();
        cr.arc(col_x, top_y + 100.0 * unit, 2.0 * unit, 0.0, 2.0 * PI);
        cr.set_source_rgba(rgb.0, rgb.1, rgb.2, alpha);
        let _ = cr.fill();
    }

    let pending_tasks = p3.tasks.iter().filter(|t| !t.completed).count();
    cr.new_sub_path();
    cr.arc(x + 73.0 * unit, top_y + 119.0 * unit, 3.0 * unit, 0.0, 2.0 * PI);
    cr.set_source_rgba(0.188, 0.780, 0.349, alpha);
    let _ = cr.fill();

    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(10.8 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
    cr.move_to(x + 82.0 * unit, top_y + 123.0 * unit);
    let _ = cr.show_text(&format!("{pending_tasks} tasks open"));

    let next_p = &p3.prayer.entries[p3.prayer.next_idx];
    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(9.8 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.45 * alpha);
    cr.move_to(x + 70.0 * unit, top_y + 139.0 * unit);
    let _ = cr.show_text(&format!("Next: {} · {}", next_p.name, next_p.hh_mm()));

    // Right Widget: BATTERY (274, 40, 204x112)
    rrect(cr, x + 274.0 * unit, top_y + 40.0 * unit, 204.0 * unit, 112.0 * unit, 14.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.06 * alpha);
    let _ = cr.fill();

    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(9.5 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.45 * alpha);
    cr.move_to(x + 286.0 * unit, top_y + 56.0 * unit);
    let _ = cr.show_text("BATTERY");

    // 52x52 ring at (292, 74) -> center (318, 100), r = 24
    let rcx = x + 318.0 * unit;
    let rcy = top_y + 100.0 * unit;
    let rr = 23.0 * unit;
    let col = if p3.battery_pct <= 20 {
        (0.961, 0.620, 0.043)
    } else {
        (0.188, 0.780, 0.349) // #30C759
    };
    cr.set_line_width(4.2 * unit);
    cr.set_line_cap(cairo::LineCap::Round);
    cr.new_sub_path();
    cr.arc(rcx, rcy, rr, 0.0, 2.0 * PI);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.12 * alpha);
    let _ = cr.stroke();
    let frac = (p3.battery_pct as f64 / 100.0).clamp(0.05, 1.0);
    cr.new_sub_path();
    cr.arc(rcx, rcy, rr, -PI * 0.5, -PI * 0.5 + frac * 2.0 * PI);
    cr.set_source_rgba(col.0, col.1, col.2, alpha);
    let _ = cr.stroke();

    // Bolt inside ring
    cr.new_sub_path();
    cr.move_to(rcx + 1.5 * unit, rcy - 7.0 * unit);
    cr.line_to(rcx - 4.5 * unit, rcy + 0.8 * unit);
    cr.line_to(rcx - 0.5 * unit, rcy + 0.8 * unit);
    cr.line_to(rcx - 1.5 * unit, rcy + 7.0 * unit);
    cr.line_to(rcx + 4.5 * unit, rcy - 1.0 * unit);
    cr.line_to(rcx + 0.5 * unit, rcy - 1.0 * unit);
    cr.close_path();
    cr.set_source_rgba(col.0, col.1, col.2, alpha);
    let _ = cr.fill();

    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(24.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
    cr.move_to(x + 358.0 * unit, top_y + 94.0 * unit);
    let _ = cr.show_text(&format!("{}%", p3.battery_pct));

    cr.set_font_size(11.0 * unit);
    cr.set_source_rgba(col.0, col.1, col.2, alpha);
    cr.move_to(x + 358.0 * unit, top_y + 111.0 * unit);
    let _ = cr.show_text(if p3.battery_charging { "Charging" } else { "On battery" });

    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(10.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.45 * alpha);
    cr.move_to(x + 358.0 * unit, top_y + 125.0 * unit);
    let _ = cr.show_text(if p3.battery_charging { "Plugged in" } else { "UPower live" });
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}
