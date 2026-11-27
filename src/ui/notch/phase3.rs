use crate::config::{FaithTradition, UmiConfig, WidgetKind, WidgetsConfig};
use crate::domain::pomodoro::{PomodoroEngine, SegmentKind};
use crate::domain::prayer_calc::PrayerSchedule;
use crate::storage::{ShelfItem, TaskItem, WaterSummary};
use crate::sys::telemetry::{format_speed, WidgetTelemetry};
use crate::ui::notch::renderer::RectF;
use crate::ui::notch::text;
use chrono::{Datelike, Duration, Local, NaiveDate, Utc};
use gtk4::cairo::{self, Context, LinearGradient};
use std::collections::HashSet;
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
    pub todo_selected_date: NaiveDate,
    pub todo_entry_open: bool,
    pub battery_pct: u8,
    pub battery_charging: bool,
    pub widgets_cfg: &'a WidgetsConfig,
    pub telemetry: &'a WidgetTelemetry,
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
    pub today_btn: RectF,
    pub prev_month_btn: RectF,
    pub next_month_btn: RectF,
    pub day_cells: Vec<(NaiveDate, RectF)>,
    pub rows: [RectF; 3],
}

pub struct WidgetsHitZones {
    pub left_tile: RectF,
    pub right_tile: RectF,
}

pub fn widgets_hit_zones(x: f64, top_y: f64, unit: f64) -> WidgetsHitZones {
    WidgetsHitZones {
        left_tile: RectF {
            x: x + 58.0 * unit,
            y: top_y + 40.0 * unit,
            w: 204.0 * unit,
            h: 112.0 * unit,
        },
        right_tile: RectF {
            x: x + 274.0 * unit,
            y: top_y + 40.0 * unit,
            w: 204.0 * unit,
            h: 112.0 * unit,
        },
    }
}

/// Compute 3-week (21-day) rolling calendar grid centered around `selected_date`.
pub fn calendar_window_dates(selected_date: NaiveDate, week_starts_monday: bool) -> [NaiveDate; 21] {
    let offset = if week_starts_monday {
        selected_date.weekday().num_days_from_monday() as i64
    } else {
        selected_date.weekday().num_days_from_sunday() as i64
    };
    let this_week_start = selected_date - Duration::days(offset);
    let grid_start = this_week_start - Duration::days(7);
    let mut out = [grid_start; 21];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = grid_start + Duration::days(i as i64);
    }
    out
}

/// Filter all tasks for a specific date (includes undated legacy tasks on `today`).
pub fn tasks_for_date<'a>(
    all: &'a [TaskItem],
    selected: NaiveDate,
    today: NaiveDate,
) -> Vec<&'a TaskItem> {
    let target = selected.format("%Y-%m-%d").to_string();
    let is_today = selected == today;
    all.iter()
        .filter(|t| match t.due_date.as_deref() {
            Some(d) => d == target,
            None => is_today,
        })
        .collect()
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

pub fn todo_hit_zones(
    x: f64,
    top_y: f64,
    unit: f64,
    selected_date: NaiveDate,
    week_starts_monday: bool,
    calendar_enabled: bool,
) -> TodoHitZones {
    if !calendar_enabled {
        return TodoHitZones {
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
            today_btn: RectF { x: 0.0, y: 0.0, w: 0.0, h: 0.0 },
            prev_month_btn: RectF { x: 0.0, y: 0.0, w: 0.0, h: 0.0 },
            next_month_btn: RectF { x: 0.0, y: 0.0, w: 0.0, h: 0.0 },
            day_cells: Vec::new(),
            rows: [
                RectF { x: x + 244.0 * unit, y: top_y + 42.0 * unit, w: 224.0 * unit, h: 30.0 * unit },
                RectF { x: x + 244.0 * unit, y: top_y + 78.0 * unit, w: 224.0 * unit, h: 30.0 * unit },
                RectF { x: x + 244.0 * unit, y: top_y + 114.0 * unit, w: 224.0 * unit, h: 30.0 * unit },
            ],
        };
    }

    let dates = calendar_window_dates(selected_date, week_starts_monday);
    let mut day_cells = Vec::with_capacity(21);
    let col_step = 22.0;
    let base_x = 50.0;
    for (idx, d) in dates.iter().enumerate() {
        let row = (idx / 7) as f64;
        let col = (idx % 7) as f64;
        day_cells.push((
            *d,
            RectF {
                x: x + (base_x + col * col_step) * unit,
                y: top_y + (76.0 + row * 25.0) * unit,
                w: 21.0 * unit,
                h: 22.0 * unit,
            },
        ));
    }

    TodoHitZones {
        prev_month_btn: RectF {
            x: x + 52.0 * unit,
            y: top_y + 34.0 * unit,
            w: 24.0 * unit,
            h: 22.0 * unit,
        },
        next_month_btn: RectF {
            x: x + 178.0 * unit,
            y: top_y + 34.0 * unit,
            w: 24.0 * unit,
            h: 22.0 * unit,
        },
        today_btn: RectF {
            x: x + 372.0 * unit,
            y: top_y + 36.0 * unit,
            w: 44.0 * unit,
            h: 20.0 * unit,
        },
        edit_btn: RectF {
            x: x + 420.0 * unit,
            y: top_y + 36.0 * unit,
            w: 22.0 * unit,
            h: 20.0 * unit,
        },
        add_btn: RectF {
            x: x + 446.0 * unit,
            y: top_y + 36.0 * unit,
            w: 46.0 * unit,
            h: 20.0 * unit,
        },
        day_cells,
        rows: [
            RectF { x: x + 238.0 * unit, y: top_y + 62.0 * unit, w: 254.0 * unit, h: 27.0 * unit },
            RectF { x: x + 238.0 * unit, y: top_y + 93.0 * unit, w: 254.0 * unit, h: 27.0 * unit },
            RectF { x: x + 238.0 * unit, y: top_y + 124.0 * unit, w: 254.0 * unit, h: 27.0 * unit },
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

/// State 09 · Multi-Faith Prayer & Devotion (6 traditions in Indonesia — FigCraft Page 5).
pub fn draw_state_09_prayer(cr: &Context, x: f64, top_y: f64, unit: f64, p3: &Phase3Data<'_>, alpha: f64) {
    let sched = p3.prayer;
    if sched.entries.is_empty() {
        return;
    }
    let next = sched.next_entry();
    let faith = sched.faith;

    // Faith accent RGB (matches FigCraft Page 5 cards 09a..09f)
    let accent = match faith {
        FaithTradition::Islam => (0.392, 0.710, 1.0),      // Sky blue
        FaithTradition::Kristen => (0.490, 0.725, 1.0),    // Soft indigo-blue
        FaithTradition::Katolik => (0.580, 0.560, 1.0),    // Liturgical indigo
        FaithTradition::Hindu => (0.980, 0.720, 0.220),    // Golden saffron
        FaithTradition::Buddha => (0.855, 0.510, 0.790),   // Lotus magenta
        FaithTradition::Khonghucu => (0.960, 0.440, 0.440), // Crimson lantern
    };

    // Compact header subtitle cleanly bounded before the center battery ring (x ~ 184)
    let (sub_x, compact_sub) = match faith {
        FaithTradition::Islam => (78.0, sched.city.as_str()),
        FaithTradition::Kristen => (92.0, "Kristen"),
        FaithTradition::Katolik => (92.0, "Katolik"),
        FaithTradition::Hindu => (108.0, "Hindu"),
        FaithTradition::Buddha => (98.0, "Buddha"),
        FaithTradition::Khonghucu => (116.0, "Imlek"),
    };
    let sub_max_w = (176.0_f64 - sub_x).max(40.0);
    text::draw_text_baseline(
        cr,
        compact_sub,
        x + sub_x * unit,
        top_y + 23.0 * unit,
        Some(sub_max_w * unit),
        10.5 * unit,
        false,
        (1.0, 1.0, 1.0, 0.48 * alpha),
    );

    // Left pane: Next label / Name / HH:MM + countdown / progress bar / feast hint
    text::draw_text_baseline(
        cr,
        sched.next_label,
        x + 54.0 * unit,
        top_y + 53.0 * unit,
        Some(160.0 * unit),
        10.5 * unit,
        false,
        (1.0, 1.0, 1.0, 0.45 * alpha),
    );

    let title_sz = if next.name.chars().count() > 14 {
        16.5
    } else if next.name.chars().count() > 10 {
        19.5
    } else {
        23.5
    };
    text::draw_text_baseline(
        cr,
        next.name,
        x + 54.0 * unit,
        top_y + 81.0 * unit,
        Some(162.0 * unit),
        title_sz * unit,
        true,
        (1.0, 1.0, 1.0, alpha),
    );

    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(13.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
    cr.move_to(x + 54.0 * unit, top_y + 104.0 * unit);
    let _ = cr.show_text(&next.hh_mm());

    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(11.5 * unit);
    cr.set_source_rgba(accent.0, accent.1, accent.2, alpha);
    cr.move_to(x + 98.0 * unit, top_y + 104.0 * unit);
    let _ = cr.show_text(&sched.countdown_text);

    // Progress bar (54, 118, 152x4)
    rrect(cr, x + 54.0 * unit, top_y + 118.0 * unit, 152.0 * unit, 4.0 * unit, 2.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.14 * alpha);
    let _ = cr.fill();
    let fw = (152.0 * sched.segment_progress).max(8.0);
    let pg = LinearGradient::new(x + 54.0 * unit, 0.0, x + 206.0 * unit, 0.0);
    pg.add_color_stop_rgba(0.0, accent.0, accent.1, accent.2, alpha);
    pg.add_color_stop_rgba(1.0, (accent.0 * 0.7).min(1.0), (accent.1 * 0.65).min(1.0), (accent.2 * 0.9).min(1.0), alpha);
    rrect(cr, x + 54.0 * unit, top_y + 118.0 * unit, fw * unit, 4.0 * unit, 2.0 * unit);
    let _ = cr.set_source(&pg);
    let _ = cr.fill();

    // Optional feast / lunar / Sunday reminder hint below progress bar
    if let Some(ref hint) = sched.feast_hint {
        text::draw_text_baseline(
            cr,
            hint,
            x + 54.0 * unit,
            top_y + 141.0 * unit,
            Some(164.0 * unit),
            9.2 * unit,
            false,
            (accent.0, accent.1, accent.2, 0.80 * alpha),
        );
    }

    // Divider (224, 40, 1x102)
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.12 * alpha);
    cr.rectangle(x + 224.0 * unit, top_y + 40.0 * unit, 1.0 * unit, 102.0 * unit);
    let _ = cr.fill();

    // Right pane adapts to selected faith tradition
    match faith {
        FaithTradition::Islam | FaithTradition::Hindu => {
            draw_prayer_sun_arc_pane(cr, x, top_y, unit, sched, accent, alpha);
        }
        FaithTradition::Katolik => {
            // 2x2 grid (4 slots: Angelus Pagi/Siang/Sore + Kompletorium)
            let coords = [(238.0, 44.0), (370.0, 44.0), (238.0, 96.0), (370.0, 96.0)];
            for (idx, entry) in sched.entries.iter().take(4).enumerate() {
                let (cx, cy) = coords[idx];
                let is_next = idx == sched.next_idx;
                let is_done = idx < sched.next_idx;
                rrect(cr, x + cx * unit, top_y + cy * unit, 126.0 * unit, 46.0 * unit, 10.0 * unit);
                if is_next {
                    cr.set_source_rgba(accent.0, accent.1, accent.2, 0.18 * alpha);
                    let _ = cr.fill_preserve();
                    cr.set_source_rgba(accent.0, accent.1, accent.2, 0.55 * alpha);
                    cr.set_line_width(1.1 * unit);
                    let _ = cr.stroke();
                } else {
                    cr.set_source_rgba(1.0, 1.0, 1.0, 0.06 * alpha);
                    let _ = cr.fill();
                }
                let mark = if is_next { "●" } else if is_done { "✓" } else { "○" };
                let label = format!("{mark} {}", entry.name);
                text::draw_text_baseline(
                    cr,
                    &label,
                    x + (cx + 10.0) * unit,
                    top_y + (cy + 19.0) * unit,
                    Some(108.0 * unit),
                    10.0 * unit,
                    is_next,
                    (1.0, 1.0, 1.0, (if is_next { 1.0 } else { 0.62 }) * alpha),
                );
                text::draw_text_baseline(
                    cr,
                    &entry.hh_mm(),
                    x + (cx + 10.0) * unit,
                    top_y + (cy + 35.0) * unit,
                    Some(80.0 * unit),
                    10.5 * unit,
                    is_next,
                    if is_next { (accent.0, accent.1, accent.2, alpha) } else { (1.0, 1.0, 1.0, 0.45 * alpha) },
                );
            }
        }
        FaithTradition::Khonghucu => {
            // 2 taller slot cards (Pagi Dupa & Sore Renungan)
            let ys = [46.0, 96.0];
            for (idx, entry) in sched.entries.iter().take(2).enumerate() {
                let cy = ys[idx];
                let is_next = idx == sched.next_idx;
                let is_done = idx < sched.next_idx;
                rrect(cr, x + 238.0 * unit, top_y + cy * unit, 258.0 * unit, 44.0 * unit, 10.0 * unit);
                if is_next {
                    cr.set_source_rgba(accent.0, accent.1, accent.2, 0.18 * alpha);
                    let _ = cr.fill_preserve();
                    cr.set_source_rgba(accent.0, accent.1, accent.2, 0.55 * alpha);
                    cr.set_line_width(1.1 * unit);
                    let _ = cr.stroke();
                } else {
                    cr.set_source_rgba(1.0, 1.0, 1.0, 0.06 * alpha);
                    let _ = cr.fill();
                }
                let mark = if is_next { "●" } else if is_done { "✓" } else { "○" };
                let lbl = format!("{mark}  {}", entry.name);
                text::draw_text_baseline(
                    cr,
                    &lbl,
                    x + 250.0 * unit,
                    top_y + (cy + 26.0) * unit,
                    Some(195.0 * unit),
                    11.0 * unit,
                    is_next,
                    (1.0, 1.0, 1.0, (if is_next { 1.0 } else { 0.62 }) * alpha),
                );
                text::draw_text_baseline(
                    cr,
                    &entry.hh_mm(),
                    x + 452.0 * unit,
                    top_y + (cy + 26.0) * unit,
                    Some(40.0 * unit),
                    11.0 * unit,
                    is_next,
                    if is_next { (1.0, 1.0, 1.0, alpha) } else { (accent.0, accent.1, accent.2, 0.72 * alpha) },
                );
            }
        }
        FaithTradition::Kristen | FaithTradition::Buddha => {
            // 3 sleek horizontal devotion rows
            let ys = [44.0, 78.0, 112.0];
            for (idx, entry) in sched.entries.iter().take(3).enumerate() {
                let cy = ys[idx];
                let is_next = idx == sched.next_idx;
                let is_done = idx < sched.next_idx;
                rrect(cr, x + 238.0 * unit, top_y + cy * unit, 258.0 * unit, 30.0 * unit, 9.0 * unit);
                if is_next {
                    cr.set_source_rgba(accent.0, accent.1, accent.2, 0.18 * alpha);
                    let _ = cr.fill_preserve();
                    cr.set_source_rgba(accent.0, accent.1, accent.2, 0.55 * alpha);
                    cr.set_line_width(1.1 * unit);
                    let _ = cr.stroke();
                } else {
                    cr.set_source_rgba(1.0, 1.0, 1.0, 0.06 * alpha);
                    let _ = cr.fill();
                }
                let mark = if is_next { "●" } else if is_done { "✓" } else { "○" };
                let lbl = format!("{mark}  {}", entry.name);
                text::draw_text_baseline(
                    cr,
                    &lbl,
                    x + 248.0 * unit,
                    top_y + (cy + 19.0) * unit,
                    Some(195.0 * unit),
                    11.0 * unit,
                    is_next,
                    (1.0, 1.0, 1.0, (if is_next { 1.0 } else { 0.62 }) * alpha),
                );
                text::draw_text_baseline(
                    cr,
                    &entry.hh_mm(),
                    x + 452.0 * unit,
                    top_y + (cy + 19.0) * unit,
                    Some(40.0 * unit),
                    11.0 * unit,
                    is_next,
                    if is_next { (1.0, 1.0, 1.0, alpha) } else { (accent.0, accent.1, accent.2, 0.72 * alpha) },
                );
            }
        }
    }
}

fn draw_prayer_sun_arc_pane(
    cr: &Context,
    x: f64,
    top_y: f64,
    unit: f64,
    sched: &PrayerSchedule,
    accent: (f64, f64, f64),
    alpha: f64,
) {
    let ax0 = x + 244.0 * unit;
    let aw = 240.0 * unit;
    let hy = top_y + 104.0 * unit;
    let amp = 42.0 * unit;

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

    // Active illuminated arc
    let p_cur = sched.sun_arc_progress.clamp(0.02, 0.98);
    let ag = LinearGradient::new(ax0, hy, ax0 + aw, hy);
    if sched.faith == FaithTradition::Hindu {
        ag.add_color_stop_rgba(0.0, 0.980, 0.560, 0.180, alpha);
        ag.add_color_stop_rgba(1.0, 0.988, 0.820, 0.300, alpha);
    } else {
        ag.add_color_stop_rgba(0.0, 0.098, 0.722, 1.0, alpha);
        ag.add_color_stop_rgba(0.6, 0.357, 0.424, 1.0, alpha);
        ag.add_color_stop_rgba(1.0, 0.961, 0.620, 0.043, alpha);
    }
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
    cr.set_source_rgba(accent.0, accent.1, accent.2, 0.34 * alpha);
    let _ = cr.fill();
    cr.new_sub_path();
    cr.arc(sx, sy, 3.8 * unit, 0.0, 2.0 * PI);
    cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
    let _ = cr.fill();

    // Labels underneath (5 for Islam, 3 for Hindu)
    let n = sched.entries.len().max(1);
    let step_x = 248.0 / n as f64;
    for (idx, entry) in sched.entries.iter().enumerate() {
        let is_next = idx == sched.next_idx;
        let a = (if is_next { 1.0 } else { 0.48 }) * alpha;
        let lx = x + (238.0 + idx as f64 * step_x) * unit;
        text::draw_text_baseline(
            cr,
            entry.name,
            lx,
            top_y + 123.0 * unit,
            Some((step_x - 2.0) * unit),
            if n <= 3 { 9.2 * unit } else { 8.8 * unit },
            is_next,
            (1.0, 1.0, 1.0, a),
        );
        text::draw_text_baseline(
            cr,
            &entry.hh_mm(),
            lx,
            top_y + 136.0 * unit,
            Some((step_x - 2.0) * unit),
            8.8 * unit,
            is_next,
            if is_next { (accent.0, accent.1, accent.2, alpha) } else { (1.0, 1.0, 1.0, a) },
        );
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

/// State 13 · Interactive Calendar + Date-Specific What-to-Do (FigCraft Page 5 `13a`/`13b`).
pub fn draw_state_13_todo(cr: &Context, x: f64, top_y: f64, unit: f64, p3: &Phase3Data<'_>, alpha: f64) {
    if !p3.widgets_cfg.show_todo_calendar {
        draw_state_13_todo_classic(cr, x, top_y, unit, p3, alpha);
        return;
    }

    let today = Local::now().date_naive();
    let sel = p3.todo_selected_date;
    let day_tasks = tasks_for_date(p3.tasks, sel, today);
    let total = day_tasks.len();
    let done = day_tasks.iter().filter(|t| t.completed).count();

    let mut dates_with_tasks: HashSet<String> = HashSet::new();
    let today_s = today.format("%Y-%m-%d").to_string();
    for t in p3.tasks {
        if let Some(ref d) = t.due_date {
            dates_with_tasks.insert(d.clone());
        } else {
            dates_with_tasks.insert(today_s.clone());
        }
    }

    let month_lbl = sel.format("%b %Y").to_string();
    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(11.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.55 * alpha);
    cr.move_to(x + 60.0 * unit, top_y + 49.0 * unit);
    let _ = cr.show_text("‹");
    cr.move_to(x + 188.0 * unit, top_y + 49.0 * unit);
    let _ = cr.show_text("›");

    cr.set_source_rgba(1.0, 1.0, 1.0, 0.95 * alpha);
    if let Ok(ext) = cr.text_extents(&month_lbl) {
        cr.move_to(x + 127.0 * unit - ext.width() * 0.5, top_y + 49.0 * unit);
        let _ = cr.show_text(&month_lbl);
    }

    let weekdays = if p3.widgets_cfg.week_starts_monday {
        ["M", "T", "W", "T", "F", "S", "S"]
    } else {
        ["S", "M", "T", "W", "T", "F", "S"]
    };
    let col_step = 22.0;
    let base_x = 50.0;
    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(9.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.42 * alpha);
    for (col, wd) in weekdays.iter().enumerate() {
        let cx = x + (base_x + col as f64 * col_step + 10.5) * unit;
        if let Ok(ext) = cr.text_extents(wd) {
            cr.move_to(cx - ext.width() * 0.5, top_y + 67.0 * unit);
            let _ = cr.show_text(wd);
        }
    }

    let grid_dates = calendar_window_dates(sel, p3.widgets_cfg.week_starts_monday);
    for (idx, d) in grid_dates.iter().enumerate() {
        let row = (idx / 7) as f64;
        let col = (idx % 7) as f64;
        let cx = x + (base_x + col * col_step + 10.5) * unit;
        let cy = top_y + (86.0 + row * 25.0) * unit;
        let is_today = *d == today;
        let is_sel = *d == sel;
        let in_month = d.month() == sel.month();
        let d_key = d.format("%Y-%m-%d").to_string();
        let has_task = dates_with_tasks.contains(&d_key);

        if is_sel {
            cr.new_sub_path();
            cr.arc(cx, cy, 9.2 * unit, 0.0, 2.0 * PI);
            cr.set_source_rgba(0.184, 0.816, 0.420, alpha);
            let _ = cr.fill();
        } else if is_today {
            cr.new_sub_path();
            cr.arc(cx, cy, 9.2 * unit, 0.0, 2.0 * PI);
            cr.set_source_rgba(0.039, 0.518, 1.0, 0.28 * alpha);
            let _ = cr.fill_preserve();
            cr.set_source_rgba(0.039, 0.518, 1.0, alpha);
            cr.set_line_width(1.3 * unit);
            let _ = cr.stroke();
        }

        let num_s = format!("{}", d.day());
        cr.select_font_face(
            "Inter",
            cairo::FontSlant::Normal,
            if is_sel || is_today { cairo::FontWeight::Bold } else { cairo::FontWeight::Normal },
        );
        cr.set_font_size(9.6 * unit);
        if is_sel {
            cr.set_source_rgba(0.02, 0.08, 0.04, alpha);
        } else if is_today {
            cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
        } else if in_month {
            cr.set_source_rgba(0.92, 0.94, 0.98, 0.85 * alpha);
        } else {
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.30 * alpha);
        }
        if let Ok(ext) = cr.text_extents(&num_s) {
            cr.move_to(cx - ext.width() * 0.5, cy + 3.4 * unit);
            let _ = cr.show_text(&num_s);
        }

        if has_task {
            cr.new_sub_path();
            cr.arc(cx, cy + 11.4 * unit, 1.7 * unit, 0.0, 2.0 * PI);
            if is_sel {
                cr.set_source_rgba(0.184, 0.816, 0.420, alpha);
            } else if is_today {
                cr.set_source_rgba(0.039, 0.518, 1.0, alpha);
            } else {
                cr.set_source_rgba(0.980, 0.720, 0.220, 0.88 * alpha);
            }
            let _ = cr.fill();
        }
    }

    cr.set_source_rgba(1.0, 1.0, 1.0, 0.12 * alpha);
    cr.rectangle(x + 224.0 * unit, top_y + 38.0 * unit, 1.0 * unit, 112.0 * unit);
    let _ = cr.fill();

    let date_hdr = sel.format("%a, %d %b").to_string();
    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(11.5 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
    cr.move_to(x + 238.0 * unit, top_y + 50.0 * unit);
    let _ = cr.show_text(&date_hdr);

    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(10.0 * unit);
    if p3.todo_entry_open {
        cr.set_source_rgba(0.490, 0.827, 0.988, alpha);
        cr.move_to(x + 312.0 * unit, top_y + 50.0 * unit);
        let _ = cr.show_text("Adding...");
    } else {
        cr.set_source_rgba(0.216, 0.827, 0.435, 0.90 * alpha);
        cr.move_to(x + 312.0 * unit, top_y + 50.0 * unit);
        let _ = cr.show_text(&format!("{done}/{total} done"));
    }

    rrect(cr, x + 372.0 * unit, top_y + 36.0 * unit, 44.0 * unit, 20.0 * unit, 7.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, if sel == today { 0.08 * alpha } else { 0.14 * alpha });
    let _ = cr.fill();
    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(9.5 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.88 * alpha);
    cr.move_to(x + 380.0 * unit, top_y + 49.5 * unit);
    let _ = cr.show_text("Today");

    rrect(cr, x + 420.0 * unit, top_y + 36.0 * unit, 22.0 * unit, 20.0 * unit, 7.0 * unit);
    if p3.todo_edit_mode {
        cr.set_source_rgba(0.937, 0.267, 0.267, 0.50 * alpha);
    } else {
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.10 * alpha);
    }
    let _ = cr.fill();
    text::draw_text_baseline(
        cr,
        if p3.todo_edit_mode { "✓" } else { "✎" },
        x + 426.0 * unit,
        top_y + 50.0 * unit,
        None,
        10.0 * unit,
        true,
        (1.0, 1.0, 1.0, 0.90 * alpha),
    );

    rrect(cr, x + 446.0 * unit, top_y + 36.0 * unit, 46.0 * unit, 20.0 * unit, 7.0 * unit);
    cr.set_source_rgba(0.184, 0.816, 0.420, 0.26 * alpha);
    let _ = cr.fill_preserve();
    cr.set_source_rgba(0.184, 0.816, 0.420, 0.75 * alpha);
    cr.set_line_width(1.0 * unit);
    let _ = cr.stroke();
    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(9.5 * unit);
    cr.set_source_rgba(0.525, 0.937, 0.675, alpha);
    cr.move_to(x + 455.0 * unit, top_y + 49.5 * unit);
    let _ = cr.show_text("+ Add");

    if p3.todo_entry_open {
        let hint = format!("Enter simpan ke {} · Esc batal", sel.format("%d %b"));
        text::draw_text_baseline(
            cr,
            &hint,
            x + 240.0 * unit,
            top_y + 107.0 * unit,
            Some(250.0 * unit),
            9.5 * unit,
            false,
            (1.0, 1.0, 1.0, 0.48 * alpha),
        );
        if let Some(first) = day_tasks.first() {
            draw_calendar_task_row(cr, x + 238.0 * unit, top_y + 124.0 * unit, 254.0 * unit, 27.0 * unit, unit, first, p3.todo_edit_mode, alpha);
        }
        return;
    }

    let ys = [62.0_f64, 93.0, 124.0];
    if day_tasks.is_empty() {
        rrect(cr, x + 238.0 * unit, top_y + 62.0 * unit, 254.0 * unit, 88.0 * unit, 11.0 * unit);
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.045 * alpha);
        let _ = cr.fill();
        let empty_msg = format!("Belum ada What to Do di {}", sel.format("%d %b"));
        text::draw_text_baseline(
            cr,
            &empty_msg,
            x + 256.0 * unit,
            top_y + 101.0 * unit,
            Some(220.0 * unit),
            11.0 * unit,
            false,
            (1.0, 1.0, 1.0, 0.50 * alpha),
        );
        text::draw_text_baseline(
            cr,
            "Klik + Add untuk menulis jadwal hari ini",
            x + 260.0 * unit,
            top_y + 119.0 * unit,
            Some(216.0 * unit),
            9.5 * unit,
            false,
            (0.216, 0.827, 0.435, 0.72 * alpha),
        );
    } else {
        for (slot, row_y) in ys.iter().enumerate() {
            let Some(task) = day_tasks.get(p3.todo_scroll + slot) else {
                continue;
            };
            draw_calendar_task_row(
                cr,
                x + 238.0 * unit,
                top_y + row_y * unit,
                254.0 * unit,
                27.0 * unit,
                unit,
                task,
                p3.todo_edit_mode,
                alpha,
            );
        }
        if day_tasks.len() > 3 {
            let max_scroll = (day_tasks.len() - 3) as f64;
            let frac = (p3.todo_scroll as f64 / max_scroll).clamp(0.0, 1.0);
            rrect(cr, x + 496.0 * unit, top_y + (64.0 + frac * 48.0) * unit, 3.0 * unit, 36.0 * unit, 1.5 * unit);
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.40 * alpha);
            let _ = cr.fill();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_calendar_task_row(
    cr: &Context,
    rx: f64,
    ry: f64,
    rw: f64,
    rh: f64,
    unit: f64,
    task: &TaskItem,
    edit_mode: bool,
    alpha: f64,
) {
    rrect(cr, rx, ry, rw, rh, 8.5 * unit);
    if task.completed {
        cr.set_source_rgba(0.157, 0.471, 0.235, 0.34 * alpha);
    } else {
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.075 * alpha);
    }
    let _ = cr.fill();

    let ccx = rx + 15.0 * unit;
    let ccy = ry + rh * 0.5;
    cr.new_sub_path();
    cr.arc(ccx, ccy, 6.8 * unit, 0.0, 2.0 * PI);
    if edit_mode {
        cr.set_source_rgba(0.937, 0.267, 0.267, 0.90 * alpha);
        let _ = cr.fill();
        cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
        cr.set_line_width(1.5 * unit);
        cr.new_sub_path();
        cr.move_to(ccx - 2.5 * unit, ccy - 2.5 * unit);
        cr.line_to(ccx + 2.5 * unit, ccy + 2.5 * unit);
        cr.move_to(ccx + 2.5 * unit, ccy - 2.5 * unit);
        cr.line_to(ccx - 2.5 * unit, ccy + 2.5 * unit);
        let _ = cr.stroke();
    } else if task.completed {
        cr.set_source_rgba(0.184, 0.816, 0.420, alpha);
        let _ = cr.fill();
        cr.new_sub_path();
        cr.move_to(ccx - 2.8 * unit, ccy + 0.2 * unit);
        cr.line_to(ccx - 0.6 * unit, ccy + 2.4 * unit);
        cr.line_to(ccx + 3.2 * unit, ccy - 2.2 * unit);
        cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
        cr.set_line_width(1.6 * unit);
        cr.set_line_cap(cairo::LineCap::Round);
        cr.set_line_join(cairo::LineJoin::Round);
        let _ = cr.stroke();
    } else {
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.35 * alpha);
        cr.set_line_width(1.3 * unit);
        let _ = cr.stroke();
    }

    let text_x = rx + 29.0 * unit;
    let text_max_w = rw - 38.0 * unit;
    let text_a = (if task.completed { 0.55 } else { 0.95 }) * alpha;
    text::draw_text_baseline(
        cr,
        &task.title,
        text_x,
        ry + rh * 0.65,
        Some(text_max_w),
        11.0 * unit,
        false,
        (1.0, 1.0, 1.0, text_a),
    );
    if task.completed {
        let tw = text::measure_text_width(cr, &task.title, 11.0 * unit, false).min(text_max_w);
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.40 * alpha);
        cr.rectangle(text_x, ry + rh * 0.52, tw, 1.0 * unit);
        let _ = cr.fill();
    }
}

fn draw_state_13_todo_classic(cr: &Context, x: f64, top_y: f64, unit: f64, p3: &Phase3Data<'_>, alpha: f64) {
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
    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(10.5 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
    cr.move_to(x + 71.0 * unit, top_y + 130.0 * unit);
    let _ = cr.show_text("+ Add Task");

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

            text::draw_text_baseline(
                cr,
                &task.title,
                x + 276.0 * unit,
                ry + 19.0 * unit,
                Some(182.0 * unit),
                11.8 * unit,
                false,
                (1.0, 1.0, 1.0, (if task.completed { 0.55 } else { 0.95 }) * alpha),
            );

            if task.completed {
                let tw = text::measure_text_width(cr, &task.title, 11.8 * unit, false).min(182.0 * unit);
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

/// State 14 · Customizable Widgets (2 Slots: Left + Right from 8 selectable widgets — FigCraft Page 6).
pub fn draw_state_14_widgets(cr: &Context, x: f64, top_y: f64, unit: f64, p3: &Phase3Data<'_>, alpha: f64) {
    draw_widget_tile(cr, p3.widgets_cfg.left, x + 58.0 * unit, top_y + 40.0 * unit, unit, p3, alpha);
    draw_widget_tile(cr, p3.widgets_cfg.right, x + 274.0 * unit, top_y + 40.0 * unit, unit, p3, alpha);
}

fn draw_widget_tile(
    cr: &Context,
    kind: WidgetKind,
    tx: f64,
    ty: f64,
    unit: f64,
    p3: &Phase3Data<'_>,
    alpha: f64,
) {
    let tw = 204.0 * unit;
    let th = 112.0 * unit;
    rrect(cr, tx, ty, tw, th, 14.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.06 * alpha);
    let _ = cr.fill();

    match kind {
        WidgetKind::ThisWeek => draw_widget_this_week(cr, tx, ty, unit, p3, alpha),
        WidgetKind::Battery => draw_widget_battery(cr, tx, ty, unit, p3, alpha),
        WidgetKind::SystemMonitor => draw_widget_system(cr, tx, ty, unit, p3, alpha),
        WidgetKind::Weather => draw_widget_weather(cr, tx, ty, unit, p3, alpha),
        WidgetKind::NetworkSpeed => draw_widget_network(cr, tx, ty, unit, p3, alpha),
        WidgetKind::WorldClock => draw_widget_world_clock(cr, tx, ty, unit, p3, alpha),
        WidgetKind::QuickNotes => draw_widget_quick_notes(cr, tx, ty, unit, p3, alpha),
        WidgetKind::DiskThermal => draw_widget_disk_thermal(cr, tx, ty, unit, p3, alpha),
    }
}

fn draw_widget_header(cr: &Context, tx: f64, ty: f64, unit: f64, label: &str, alpha: f64) {
    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(9.2 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.45 * alpha);
    cr.move_to(tx + 12.0 * unit, ty + 16.0 * unit);
    let _ = cr.show_text(label);
}

fn draw_widget_this_week(cr: &Context, tx: f64, ty: f64, unit: f64, p3: &Phase3Data<'_>, alpha: f64) {
    draw_widget_header(cr, tx, ty, unit, "THIS WEEK", alpha);
    let today = Local::now().date_naive();
    let offset = if p3.widgets_cfg.week_starts_monday {
        today.weekday().num_days_from_monday() as i64
    } else {
        today.weekday().num_days_from_sunday() as i64
    };
    let start = today - Duration::days(offset);
    let day_letters = if p3.widgets_cfg.week_starts_monday {
        ["M", "T", "W", "T", "F", "S", "S"]
    } else {
        ["S", "M", "T", "W", "T", "F", "S"]
    };

    for i in 0..7i64 {
        let d = start + Duration::days(i);
        let col_x = tx + (19.0 + i as f64 * 25.5) * unit;
        let is_today = i == offset;

        cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
        cr.set_font_size(9.0 * unit);
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.45 * alpha);
        cr.move_to(col_x - 3.5 * unit, ty + 32.0 * unit);
        let _ = cr.show_text(day_letters[i as usize]);

        if is_today {
            cr.new_sub_path();
            cr.arc(col_x, ty + 45.0 * unit, 9.0 * unit, 0.0, 2.0 * PI);
            cr.set_source_rgba(0.039, 0.518, 1.0, alpha);
            let _ = cr.fill();
        }

        let num_s = format!("{}", d.day());
        cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        cr.set_font_size(10.5 * unit);
        cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
        if let Ok(ext) = cr.text_extents(&num_s) {
            cr.move_to(col_x - ext.width() * 0.5, ty + 49.0 * unit);
            let _ = cr.show_text(&num_s);
        }

        let d_key = d.format("%Y-%m-%d").to_string();
        let has_t = p3.tasks.iter().any(|t| !t.completed && (t.due_date.as_deref() == Some(&d_key) || (is_today && t.due_date.is_none())));
        if has_t {
            cr.new_sub_path();
            cr.arc(col_x, ty + 60.0 * unit, 2.0 * unit, 0.0, 2.0 * PI);
            cr.set_source_rgba(0.188, 0.780, 0.349, alpha);
            let _ = cr.fill();
        }
    }

    let pending_tasks = p3.tasks.iter().filter(|t| !t.completed).count();
    cr.new_sub_path();
    cr.arc(tx + 15.0 * unit, ty + 79.0 * unit, 3.0 * unit, 0.0, 2.0 * PI);
    cr.set_source_rgba(0.188, 0.780, 0.349, alpha);
    let _ = cr.fill();

    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(10.8 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
    cr.move_to(tx + 24.0 * unit, ty + 83.0 * unit);
    let _ = cr.show_text(&format!("{pending_tasks} tasks open"));

    if !p3.prayer.entries.is_empty() {
        let next_p = p3.prayer.next_entry();
        text::draw_text_baseline(
            cr,
            &format!("Next: {} · {}", next_p.name, next_p.hh_mm()),
            tx + 12.0 * unit,
            ty + 99.0 * unit,
            Some(182.0 * unit),
            9.5 * unit,
            false,
            (1.0, 1.0, 1.0, 0.45 * alpha),
        );
    }
}

fn draw_widget_battery(cr: &Context, tx: f64, ty: f64, unit: f64, p3: &Phase3Data<'_>, alpha: f64) {
    draw_widget_header(cr, tx, ty, unit, "BATTERY", alpha);
    let rcx = tx + 44.0 * unit;
    let rcy = ty + 60.0 * unit;
    let rr = 23.0 * unit;
    let col = if p3.battery_pct <= 20 {
        (0.961, 0.620, 0.043)
    } else {
        (0.188, 0.780, 0.349)
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
    cr.move_to(tx + 84.0 * unit, ty + 54.0 * unit);
    let _ = cr.show_text(&format!("{}%", p3.battery_pct));

    cr.set_font_size(11.0 * unit);
    cr.set_source_rgba(col.0, col.1, col.2, alpha);
    cr.move_to(tx + 84.0 * unit, ty + 71.0 * unit);
    let _ = cr.show_text(if p3.battery_charging { "Charging" } else { "On battery" });

    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(10.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.45 * alpha);
    cr.move_to(tx + 84.0 * unit, ty + 85.0 * unit);
    let _ = cr.show_text(if p3.battery_charging { "Plugged in" } else { "UPower live" });
}

fn draw_widget_system(cr: &Context, tx: f64, ty: f64, unit: f64, p3: &Phase3Data<'_>, alpha: f64) {
    let tel = p3.telemetry;
    draw_widget_header(cr, tx, ty, unit, "SYSTEM MONITOR", alpha);

    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(10.2 * unit);
    cr.set_source_rgba(0.92, 0.95, 1.0, 0.90 * alpha);
    cr.move_to(tx + 12.0 * unit, ty + 37.0 * unit);
    let _ = cr.show_text("CPU");
    cr.set_source_rgba(0.220, 0.741, 0.973, alpha);
    cr.move_to(tx + 162.0 * unit, ty + 37.0 * unit);
    let _ = cr.show_text(&format!("{:.0}%", tel.cpu_pct));

    rrect(cr, tx + 12.0 * unit, ty + 43.0 * unit, 180.0 * unit, 5.5 * unit, 2.8 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.12 * alpha);
    let _ = cr.fill();
    rrect(cr, tx + 12.0 * unit, ty + 43.0 * unit, (180.0 * (tel.cpu_pct / 100.0)).max(5.0) * unit, 5.5 * unit, 2.8 * unit);
    cr.set_source_rgba(0.220, 0.741, 0.973, alpha);
    let _ = cr.fill();

    cr.set_source_rgba(0.92, 0.95, 1.0, 0.90 * alpha);
    cr.move_to(tx + 12.0 * unit, ty + 67.0 * unit);
    let _ = cr.show_text(&format!("RAM · {:.1} / {:.0} GB", tel.ram_used_gb, tel.ram_total_gb));
    cr.set_source_rgba(0.659, 0.333, 0.969, alpha);
    cr.move_to(tx + 162.0 * unit, ty + 67.0 * unit);
    let _ = cr.show_text(&format!("{:.0}%", tel.ram_pct));

    rrect(cr, tx + 12.0 * unit, ty + 73.0 * unit, 180.0 * unit, 5.5 * unit, 2.8 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.12 * alpha);
    let _ = cr.fill();
    rrect(cr, tx + 12.0 * unit, ty + 73.0 * unit, (180.0 * (tel.ram_pct / 100.0)).max(5.0) * unit, 5.5 * unit, 2.8 * unit);
    cr.set_source_rgba(0.659, 0.333, 0.969, alpha);
    let _ = cr.fill();

    let temp_s = tel
        .cpu_temp_c
        .map(|t| format!(" · {:.0}°C", t))
        .unwrap_or_default();
    text::draw_text_baseline(
        cr,
        &format!("Load {:.2}{temp_s}", tel.load_avg),
        tx + 12.0 * unit,
        ty + 98.0 * unit,
        Some(180.0 * unit),
        9.4 * unit,
        false,
        (1.0, 1.0, 1.0, 0.48 * alpha),
    );
}

fn draw_widget_weather(cr: &Context, tx: f64, ty: f64, unit: f64, p3: &Phase3Data<'_>, alpha: f64) {
    let w = &p3.telemetry.weather;
    let city_hdr = if w.city.is_empty() {
        format!("WEATHER · {}", p3.prayer.city.to_uppercase())
    } else {
        format!("WEATHER · {}", w.city.to_uppercase())
    };
    draw_widget_header(cr, tx, ty, unit, &trunc(&city_hdr, 22), alpha);

    cr.new_sub_path();
    cr.arc(tx + 28.0 * unit, ty + 48.0 * unit, 12.0 * unit, 0.0, 2.0 * PI);
    cr.set_source_rgba(0.961, 0.620, 0.043, alpha);
    let _ = cr.fill();
    rrect(cr, tx + 22.0 * unit, ty + 50.0 * unit, 30.0 * unit, 14.0 * unit, 7.0 * unit);
    cr.set_source_rgba(0.886, 0.910, 0.941, 0.95 * alpha);
    let _ = cr.fill();

    let temp_txt = if w.ready {
        format!("{:.0}°C", w.temp_c)
    } else {
        "--°C".to_string()
    };
    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(24.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
    cr.move_to(tx + 66.0 * unit, ty + 55.0 * unit);
    let _ = cr.show_text(&temp_txt);

    text::draw_text_baseline(
        cr,
        w.description_id(),
        tx + 66.0 * unit,
        ty + 74.0 * unit,
        Some(128.0 * unit),
        10.5 * unit,
        false,
        (0.992, 0.902, 0.541, alpha),
    );

    let foot = if w.ready {
        format!("H: {:.0}°  L: {:.0}°  ·  Hum {}%", w.high_c, w.low_c, w.humidity_pct)
    } else {
        "Open-Meteo · 30m cache".to_string()
    };
    text::draw_text_baseline(
        cr,
        &foot,
        tx + 12.0 * unit,
        ty + 98.0 * unit,
        Some(182.0 * unit),
        9.4 * unit,
        false,
        (1.0, 1.0, 1.0, 0.48 * alpha),
    );
}

fn draw_widget_network(cr: &Context, tx: f64, ty: f64, unit: f64, p3: &Phase3Data<'_>, alpha: f64) {
    let tel = p3.telemetry;
    draw_widget_header(cr, tx, ty, unit, "NETWORK SPEED", alpha);

    text::draw_text_baseline(
        cr,
        "↓ Down",
        tx + 12.0 * unit,
        ty + 44.0 * unit,
        Some(70.0 * unit),
        11.0 * unit,
        true,
        (0.184, 0.816, 0.420, alpha),
    );
    text::draw_text_baseline(
        cr,
        &format_speed(tel.net_down_bps),
        tx + 92.0 * unit,
        ty + 45.0 * unit,
        Some(104.0 * unit),
        15.0 * unit,
        true,
        (1.0, 1.0, 1.0, alpha),
    );

    text::draw_text_baseline(
        cr,
        "↑ Up",
        tx + 12.0 * unit,
        ty + 72.0 * unit,
        Some(70.0 * unit),
        11.0 * unit,
        true,
        (0.961, 0.620, 0.043, alpha),
    );
    text::draw_text_baseline(
        cr,
        &format_speed(tel.net_up_bps),
        tx + 92.0 * unit,
        ty + 73.0 * unit,
        Some(104.0 * unit),
        14.0 * unit,
        true,
        (0.886, 0.910, 0.941, alpha),
    );

    text::draw_text_baseline(
        cr,
        &tel.net_iface,
        tx + 12.0 * unit,
        ty + 98.0 * unit,
        Some(182.0 * unit),
        9.5 * unit,
        false,
        (1.0, 1.0, 1.0, 0.48 * alpha),
    );
}

fn draw_widget_world_clock(cr: &Context, tx: f64, ty: f64, unit: f64, p3: &Phase3Data<'_>, alpha: f64) {
    draw_widget_header(cr, tx, ty, unit, "WORLD CLOCK", alpha);
    let utc_now = Utc::now();
    let defaults = [
        ("Surabaya (WIB)".to_string(), 7),
        ("Tokyo (JST)".to_string(), 9),
        ("London (UTC)".to_string(), 0),
    ];
    let zones = if p3.widgets_cfg.world_clock_zones.is_empty() {
        &defaults[..]
    } else {
        &p3.widgets_cfg.world_clock_zones[..]
    };

    for (idx, (label, offset_h)) in zones.iter().take(3).enumerate() {
        let ry = ty + (39.0 + idx as f64 * 26.0) * unit;
        let shifted = utc_now + Duration::hours(*offset_h as i64);
        let hhmm = shifted.format("%H:%M").to_string();
        text::draw_text_baseline(
            cr,
            label,
            tx + 12.0 * unit,
            ry,
            Some(132.0 * unit),
            10.5 * unit,
            idx == 0,
            (1.0, 1.0, 1.0, (if idx == 0 { 0.96 } else { 0.72 }) * alpha),
        );
        text::draw_text_baseline(
            cr,
            &hhmm,
            tx + 150.0 * unit,
            ry,
            Some(46.0 * unit),
            11.5 * unit,
            true,
            if idx == 0 {
                (0.220, 0.741, 0.973, alpha)
            } else {
                (0.886, 0.910, 0.941, 0.85 * alpha)
            },
        );
    }
}

fn draw_widget_quick_notes(cr: &Context, tx: f64, ty: f64, unit: f64, p3: &Phase3Data<'_>, alpha: f64) {
    draw_widget_header(cr, tx, ty, unit, "QUICK NOTES", alpha);
    text::draw_text_baseline(
        cr,
        "Pinned",
        tx + 158.0 * unit,
        ty + 16.0 * unit,
        None,
        8.8 * unit,
        true,
        (0.984, 0.749, 0.141, alpha),
    );
    text::draw_text_wrapped(
        cr,
        &p3.telemetry.quick_note,
        tx + 12.0 * unit,
        ty + 27.0 * unit,
        180.0 * unit,
        3,
        10.2 * unit,
        false,
        (0.94, 0.96, 1.0, 0.90 * alpha),
    );
    text::draw_text_baseline(
        cr,
        "Klik untuk copy · Edit di Settings",
        tx + 12.0 * unit,
        ty + 100.0 * unit,
        Some(182.0 * unit),
        8.8 * unit,
        false,
        (1.0, 1.0, 1.0, 0.42 * alpha),
    );
}

fn draw_widget_disk_thermal(cr: &Context, tx: f64, ty: f64, unit: f64, p3: &Phase3Data<'_>, alpha: f64) {
    let tel = p3.telemetry;
    draw_widget_header(cr, tx, ty, unit, "DISK & THERMAL", alpha);

    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(10.2 * unit);
    cr.set_source_rgba(0.92, 0.95, 1.0, 0.90 * alpha);
    cr.move_to(tx + 12.0 * unit, ty + 37.0 * unit);
    let _ = cr.show_text("SSD / (Root)");
    cr.set_source_rgba(0.204, 0.827, 0.600, alpha);
    cr.move_to(tx + 162.0 * unit, ty + 37.0 * unit);
    let _ = cr.show_text(&format!("{:.0}%", tel.disk_pct));

    rrect(cr, tx + 12.0 * unit, ty + 43.0 * unit, 180.0 * unit, 5.5 * unit, 2.8 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.12 * alpha);
    let _ = cr.fill();
    rrect(cr, tx + 12.0 * unit, ty + 43.0 * unit, (180.0 * (tel.disk_pct / 100.0)).max(5.0) * unit, 5.5 * unit, 2.8 * unit);
    cr.set_source_rgba(0.204, 0.827, 0.600, alpha);
    let _ = cr.fill();

    let free_gb = (tel.disk_total_gb - tel.disk_used_gb).max(0.0);
    text::draw_text_baseline(
        cr,
        &format!("{:.0} GB dipakai · {:.0} GB sisa", tel.disk_used_gb, free_gb),
        tx + 12.0 * unit,
        ty + 67.0 * unit,
        Some(182.0 * unit),
        9.6 * unit,
        false,
        (1.0, 1.0, 1.0, 0.55 * alpha),
    );

    let cpu_s = tel
        .cpu_temp_c
        .map(|t| format!("CPU {:.0}°C", t))
        .unwrap_or_else(|| "CPU --°C".to_string());
    let nvme_s = tel
        .nvme_temp_c
        .map(|t| format!("  ·  NVMe {:.0}°C", t))
        .unwrap_or_default();
    text::draw_text_baseline(
        cr,
        &format!("{cpu_s}{nvme_s}"),
        tx + 12.0 * unit,
        ty + 94.0 * unit,
        Some(182.0 * unit),
        10.8 * unit,
        true,
        (0.984, 0.749, 0.141, alpha),
    );
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
