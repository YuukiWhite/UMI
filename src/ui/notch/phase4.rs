use crate::ai::AiResponse;
use crate::quota::{QuotaSnapshot, QuotaStatus};
use crate::ui::notch::renderer::RectF;
use crate::ui::notch::text;
use chrono::Local;
use gtk4::cairo::{self, Context, LinearGradient};
use std::f64::consts::PI;

pub struct Phase4Data<'a> {
    pub translate_mode: bool,
    pub ai_loading: bool,
    pub ai_response: Option<&'a AiResponse>,
    pub ai_copied_flash: bool,
    pub quotas: &'a [QuotaSnapshot],
}

pub struct AiPromptHitZones {
    pub input_box: RectF,
    pub send_btn: RectF,
    pub chip_summarize: RectF,
    pub chip_translate: RectF,
    pub chip_ocr: RectF,
    pub chip_math: RectF,
}

pub struct AiAnswerHitZones {
    pub mode_ask: RectF,
    pub mode_translate: RectF,
    pub copy_btn: RectF,
    pub retry_btn: RectF,
    pub followup_btn: RectF,
}

pub struct AiLimitsHitZones {
    pub refresh_pill: RectF,
}

pub fn ai_prompt_hit_zones(x: f64, top_y: f64, unit: f64) -> AiPromptHitZones {
    AiPromptHitZones {
        input_box: RectF { x: x + 34.0 * unit, y: top_y + 44.0 * unit, w: 438.0 * unit, h: 42.0 * unit },
        send_btn: RectF { x: x + 474.0 * unit, y: top_y + 49.0 * unit, w: 32.0 * unit, h: 32.0 * unit },
        chip_summarize: RectF { x: x + 34.0 * unit, y: top_y + 98.0 * unit, w: 92.0 * unit, h: 24.0 * unit },
        chip_translate: RectF { x: x + 132.0 * unit, y: top_y + 98.0 * unit, w: 108.0 * unit, h: 24.0 * unit },
        chip_ocr: RectF { x: x + 246.0 * unit, y: top_y + 98.0 * unit, w: 118.0 * unit, h: 24.0 * unit },
        chip_math: RectF { x: x + 370.0 * unit, y: top_y + 98.0 * unit, w: 106.0 * unit, h: 24.0 * unit },
    }
}

pub fn ai_answer_hit_zones(x: f64, top_y: f64, unit: f64) -> AiAnswerHitZones {
    AiAnswerHitZones {
        mode_ask: RectF { x: x + 209.0 * unit, y: top_y + 9.0 * unit, w: 56.0 * unit, h: 22.0 * unit },
        mode_translate: RectF { x: x + 265.0 * unit, y: top_y + 9.0 * unit, w: 72.0 * unit, h: 22.0 * unit },
        copy_btn: RectF { x: x + 34.0 * unit, y: top_y + 136.0 * unit, w: 56.0 * unit, h: 22.0 * unit },
        retry_btn: RectF { x: x + 96.0 * unit, y: top_y + 136.0 * unit, w: 56.0 * unit, h: 22.0 * unit },
        followup_btn: RectF { x: x + 158.0 * unit, y: top_y + 136.0 * unit, w: 108.0 * unit, h: 22.0 * unit },
    }
}

pub fn ai_limits_hit_zones(x: f64, top_y: f64, unit: f64) -> AiLimitsHitZones {
    AiLimitsHitZones {
        refresh_pill: RectF { x: x + 458.0 * unit, y: top_y + 10.0 * unit, w: 64.0 * unit, h: 18.0 * unit },
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

fn draw_sparkle(cr: &Context, cx: f64, cy: f64, r: f64, rgb: (f64, f64, f64), alpha: f64) {
    cr.new_sub_path();
    cr.move_to(cx, cy - r);
    cr.curve_to(cx + r * 0.16, cy - r * 0.16, cx + r * 0.16, cy - r * 0.16, cx + r, cy);
    cr.curve_to(cx + r * 0.16, cy + r * 0.16, cx + r * 0.16, cy + r * 0.16, cx, cy + r);
    cr.curve_to(cx - r * 0.16, cy + r * 0.16, cx - r * 0.16, cy + r * 0.16, cx - r, cy);
    cr.curve_to(cx - r * 0.16, cy - r * 0.16, cx - r * 0.16, cy - r * 0.16, cx, cy - r);
    cr.close_path();
    cr.set_source_rgba(rgb.0, rgb.1, rgb.2, alpha);
    let _ = cr.fill();
}

fn draw_ai_bottom_glow(cr: &Context, x: f64, top_y: f64, w: f64, h: f64, alpha: f64) {
    let g = LinearGradient::new(x + 40.0, top_y + h, x + w - 40.0, top_y + h);
    g.add_color_stop_rgba(0.0, 0.216, 0.741, 0.969, 0.16 * alpha);
    g.add_color_stop_rgba(0.5, 0.388, 0.400, 0.945, 0.20 * alpha);
    g.add_color_stop_rgba(1.0, 0.659, 0.333, 0.965, 0.16 * alpha);
    let _ = cr.set_source(&g);
    cr.rectangle(x, top_y + h * 0.62, w, h * 0.38);
    let _ = cr.fill();

    let fade = LinearGradient::new(x, top_y + h * 0.62, x, top_y + h);
    fade.add_color_stop_rgba(0.0, 0.0, 0.0, 0.0, 1.0);
    fade.add_color_stop_rgba(0.7, 0.0, 0.0, 0.0, 0.35);
    fade.add_color_stop_rgba(1.0, 0.0, 0.0, 0.0, 0.0);
    let _ = cr.set_source(&fade);
    cr.rectangle(x, top_y + h * 0.62, w, h * 0.38);
    let _ = cr.fill();
}

/// State 15 · Ask UMI AI — Prompt — 1:1 FigCraft Card 15 (546x182 @ 1.3x, no static Gemini badge).
pub fn draw_state_15_ai_prompt(
    cr: &Context,
    x: f64,
    top_y: f64,
    w: f64,
    h: f64,
    unit: f64,
    p4: &Phase4Data<'_>,
    entry_visible: bool,
    alpha: f64,
) {
    draw_ai_bottom_glow(cr, x, top_y, w, h, alpha);

    // Header sparkle icon at (24, 12, 13x13)
    draw_sparkle(cr, x + 30.5 * unit, top_y + 18.5 * unit, 6.2 * unit, (0.216, 0.741, 0.969), alpha);

    // Prompt Input Box (34, 44, 478x42) with Sky-Indigo-Violet gradient stroke
    let ix = x + 34.0 * unit;
    let iy = top_y + 44.0 * unit;
    let iw = 478.0 * unit;
    let ih = 42.0 * unit;
    rrect(cr, ix, iy, iw, ih, 13.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.055 * alpha);
    let _ = cr.fill();

    let border_g = LinearGradient::new(ix, iy, ix + iw, iy);
    border_g.add_color_stop_rgba(0.0, 0.216, 0.741, 0.969, 0.85 * alpha);
    border_g.add_color_stop_rgba(0.5, 0.388, 0.400, 0.945, 0.85 * alpha);
    border_g.add_color_stop_rgba(1.0, 0.659, 0.333, 0.965, 0.85 * alpha);
    rrect(cr, ix, iy, iw, ih, 13.0 * unit);
    let _ = cr.set_source(&border_g);
    cr.set_line_width(1.35 * unit);
    let _ = cr.stroke();

    // Sparkle inside input left
    draw_sparkle(cr, x + 53.5 * unit, top_y + 65.0 * unit, 5.2 * unit, (0.490, 0.827, 0.988), alpha);

    // Fallback placeholder text when GTK Entry isn't focused/visible yet (e.g. headless preview)
    if !entry_visible {
        cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
        cr.set_font_size(12.5 * unit);
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.88 * alpha);
        cr.move_to(x + 68.0 * unit, top_y + 69.5 * unit);
        let txt = if p4.translate_mode {
            "Type text to translate into Indonesian..."
        } else {
            "Explain quantum tunneling simply"
        };
        let _ = cr.show_text(txt);
    }

    // Circular Send Button ↑ (476, 51, 28x28)
    let scx = x + 490.0 * unit;
    let scy = top_y + 65.0 * unit;
    let sr = 14.0 * unit;
    let sg = LinearGradient::new(scx - sr, scy - sr, scx + sr, scy + sr);
    sg.add_color_stop_rgba(0.0, 0.216, 0.741, 0.969, alpha);
    sg.add_color_stop_rgba(1.0, 0.388, 0.400, 0.945, alpha);
    cr.new_sub_path();
    cr.arc(scx, scy, sr, 0.0, 2.0 * PI);
    let _ = cr.set_source(&sg);
    let _ = cr.fill();

    cr.new_sub_path();
    cr.move_to(scx, scy + 4.8 * unit);
    cr.line_to(scx, scy - 4.8 * unit);
    cr.move_to(scx - 4.2 * unit, scy - 0.8 * unit);
    cr.line_to(scx, scy - 5.0 * unit);
    cr.line_to(scx + 4.2 * unit, scy - 0.8 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
    cr.set_line_width(1.8 * unit);
    cr.set_line_cap(cairo::LineCap::Round);
    cr.set_line_join(cairo::LineJoin::Round);
    let _ = cr.stroke();

    // 4 Quick Chips (exact FigCraft positions: 34, 132, 246, 370, y=98, h=24)
    let chips: [(f64, f64, &str, bool, bool); 4] = [
        (34.0, 92.0, "Summarize", false, false),
        (132.0, 108.0, "Translate · Tab", p4.translate_mode, false),
        (246.0, 118.0, "OCR · Ctrl+Alt+T", false, false),
        (370.0, 106.0, "100usd to idr=", false, true),
    ];
    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(10.5 * unit);
    for (cx_r, cw_r, lbl, active_chip, is_sky) in chips {
        rrect(cr, x + cx_r * unit, top_y + 98.0 * unit, cw_r * unit, 24.0 * unit, 8.0 * unit);
        if active_chip {
            cr.set_source_rgba(0.388, 0.400, 0.945, 0.45 * alpha);
        } else {
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.065 * alpha);
        }
        let _ = cr.fill();

        if is_sky {
            cr.set_source_rgba(0.490, 0.827, 0.988, alpha); // #7DD3FC
        } else {
            cr.set_source_rgba(0.925, 0.933, 0.965, alpha);
        }
        if let Ok(ext) = cr.text_extents(lbl) {
            cr.move_to(
                x + (cx_r + cw_r * 0.5) * unit - ext.width() * 0.5,
                top_y + 113.5 * unit,
            );
            let _ = cr.show_text(lbl);
        }
    }

    // Footer hint (34, 136)
    cr.set_font_size(10.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.40 * alpha);
    cr.move_to(x + 34.0 * unit, top_y + 146.0 * unit);
    let _ = cr.show_text("Press Tab to switch to Translate · Ctrl+Alt+Space to toggle");
}

/// State 16 · Ask UMI AI — Answer & Translate — 1:1 FigCraft Card 16 (546x182 @ 1.3x).
pub fn draw_state_16_ai_answer(
    cr: &Context,
    x: f64,
    top_y: f64,
    w: f64,
    h: f64,
    unit: f64,
    p4: &Phase4Data<'_>,
    alpha: f64,
) {
    draw_ai_bottom_glow(cr, x, top_y, w, h, alpha);

    // Header: Sparkle + UMI AI left, Ask | Translate switcher center, Time right
    draw_sparkle(cr, x + 30.5 * unit, top_y + 18.5 * unit, 6.2 * unit, (0.216, 0.741, 0.969), alpha);
    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(13.5 * unit);
    cr.set_source_rgba(0.490, 0.827, 0.988, alpha);
    cr.move_to(x + 42.0 * unit, top_y + 23.0 * unit);
    let _ = cr.show_text("UMI AI");

    // Mode Pill (209, 9, 128x22)
    rrect(cr, x + 209.0 * unit, top_y + 9.0 * unit, 128.0 * unit, 22.0 * unit, 11.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.08 * alpha);
    let _ = cr.fill();

    if p4.translate_mode {
        rrect(cr, x + 263.0 * unit, top_y + 11.0 * unit, 72.0 * unit, 18.0 * unit, 9.0 * unit);
    } else {
        rrect(cr, x + 211.0 * unit, top_y + 11.0 * unit, 52.0 * unit, 18.0 * unit, 9.0 * unit);
    }
    cr.set_source_rgba(0.388, 0.400, 0.945, alpha); // #6366F1
    let _ = cr.fill();

    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(10.5 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, (if !p4.translate_mode { 1.0 } else { 0.60 }) * alpha);
    cr.move_to(x + 226.0 * unit, top_y + 23.5 * unit);
    let _ = cr.show_text("Ask");
    cr.set_source_rgba(1.0, 1.0, 1.0, (if p4.translate_mode { 1.0 } else { 0.60 }) * alpha);
    cr.move_to(x + 274.0 * unit, top_y + 23.5 * unit);
    let _ = cr.show_text("Translate");

    let time_s = Local::now().format("%H:%M").to_string();
    cr.set_font_size(12.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
    cr.move_to(x + 482.0 * unit, top_y + 23.0 * unit);
    let _ = cr.show_text(&time_s);

    // Prompt echo (34, 38)
    let echo = if p4.ai_loading {
        "UMI AI is processing your request..."
    } else if let Some(resp) = p4.ai_response {
        resp.prompt_echo.as_str()
    } else {
        "You: Ask UMI AI"
    };
    text::draw_text_baseline(
        cr,
        echo,
        x + 34.0 * unit,
        top_y + 47.0 * unit,
        Some(474.0 * unit),
        11.0 * unit,
        false,
        (1.0, 1.0, 1.0, 0.45 * alpha),
    );

    // Answer Card (34, 56, 478x72)
    rrect(cr, x + 34.0 * unit, top_y + 56.0 * unit, 478.0 * unit, 72.0 * unit, 12.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.045 * alpha);
    let _ = cr.fill();

    let answer_body = if p4.ai_loading {
        "Thinking... (fetching response securely via KWallet key)"
    } else if let Some(resp) = p4.ai_response {
        resp.answer.as_str()
    } else {
        "No answer yet."
    };

    text::draw_text_wrapped(
        cr,
        answer_body,
        x + 46.0 * unit,
        top_y + 64.0 * unit,
        454.0 * unit,
        3,
        11.5 * unit,
        false,
        (0.925, 0.933, 0.965, alpha),
    );

    // Bottom buttons: Copy (34, 136, 56x22), Retry (96, 136, 56x22), Ask follow-up... (158, 136, 108x22)
    rrect(cr, x + 34.0 * unit, top_y + 136.0 * unit, 56.0 * unit, 22.0 * unit, 8.0 * unit);
    if p4.ai_copied_flash {
        cr.set_source_rgba(0.180, 0.859, 0.420, 0.30 * alpha);
    } else {
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.08 * alpha);
    }
    let _ = cr.fill();
    cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(10.5 * unit);
    cr.set_source_rgba(0.925, 0.933, 0.965, alpha);
    cr.move_to(x + 45.0 * unit, top_y + 150.5 * unit);
    let _ = cr.show_text(if p4.ai_copied_flash { "Copied" } else { "Copy" });

    rrect(cr, x + 96.0 * unit, top_y + 136.0 * unit, 56.0 * unit, 22.0 * unit, 8.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.08 * alpha);
    let _ = cr.fill();
    cr.set_source_rgba(0.925, 0.933, 0.965, alpha);
    cr.move_to(x + 110.0 * unit, top_y + 150.5 * unit);
    let _ = cr.show_text("Retry");

    rrect(cr, x + 158.0 * unit, top_y + 136.0 * unit, 108.0 * unit, 22.0 * unit, 8.0 * unit);
    cr.set_source_rgba(0.216, 0.741, 0.969, 0.14 * alpha);
    let _ = cr.fill();
    cr.set_source_rgba(0.490, 0.827, 0.988, alpha);
    cr.move_to(x + 173.0 * unit, top_y + 150.5 * unit);
    let _ = cr.show_text("Ask follow-up...");

    // Metadata latency right
    let meta = if p4.ai_loading {
        "UMI AI · ...".to_string()
    } else if let Some(resp) = p4.ai_response {
        format!("{} · {:.1}s", resp.provider_label, resp.latency_ms as f64 / 1000.0)
    } else {
        "UMI AI".to_string()
    };
    cr.set_font_size(10.0 * unit);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.40 * alpha);
    if let Ok(ext) = cr.text_extents(&meta) {
        cr.move_to(x + w - 34.0 * unit - ext.width(), top_y + 150.5 * unit);
        let _ = cr.show_text(&meta);
    }
}

/// State 17 · AI Limits — Subscription Quota — adaptive 1/2/3 card layout matching FigCraft Card 17.
pub fn draw_state_17_ai_limits(
    cr: &Context,
    x: f64,
    top_y: f64,
    unit: f64,
    p4: &Phase4Data<'_>,
    alpha: f64,
) {
    let count = p4.quotas.len().min(3);
    if count == 0 {
        cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
        cr.set_font_size(11.5 * unit);
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.48 * alpha);
        let msg = "Enable providers in Settings → AI & Shortcuts";
        if let Ok(ext) = cr.text_extents(msg) {
            cr.move_to(x + 273.0 * unit - ext.width() * 0.5, top_y + 100.0 * unit);
            let _ = cr.show_text(msg);
        }
        return;
    }

    let (xs, cw_raw): (&[f64], f64) = match count {
        1 => (&[123.0], 300.0),
        2 => (&[54.0, 280.0], 212.0),
        _ => (&[54.0, 204.0, 354.0], 140.0),
    };
    let max_plan_chars = match count {
        1 => 34,
        2 => 26,
        _ => 18,
    };
    let max_reset_chars = match count {
        1 => 34,
        2 => 26,
        _ => 20,
    };

    for (idx, cx_r) in xs.iter().enumerate() {
        let Some(snap) = p4.quotas.get(idx) else {
            continue;
        };
        let cx = x + cx_r * unit;
        let cy = top_y + 42.0 * unit;
        let cw = cw_raw * unit;
        let ch = 110.0 * unit;

        let is_low = matches!(snap.status, QuotaStatus::Low | QuotaStatus::Exhausted);
        let is_unavail = snap.status == QuotaStatus::Unavailable;
        let (c1, c2) = provider_gradient(&snap.provider_id, is_low);

        rrect(cr, cx, cy, cw, ch, 12.0 * unit);
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.045 * alpha);
        let _ = cr.fill();
        rrect(cr, cx, cy, cw, ch, 12.0 * unit);
        cr.set_source_rgba(c1.0, c1.1, c1.2, (if is_low { 0.34 } else { 0.16 }) * alpha);
        cr.set_line_width(1.0);
        let _ = cr.stroke();

        // Status dot
        cr.new_sub_path();
        cr.arc(cx + 14.5 * unit, cy + 16.0 * unit, 3.5 * unit, 0.0, 2.0 * PI);
        cr.set_source_rgba(c1.0, c1.1, c1.2, (if is_unavail { 0.4 } else { 1.0 }) * alpha);
        let _ = cr.fill();

        // Provider Name
        cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        cr.set_font_size(12.5 * unit);
        cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
        cr.move_to(cx + 23.0 * unit, cy + 20.0 * unit);
        let _ = cr.show_text(&trunc(&snap.display_name, if count == 3 { 13 } else { 20 }));

        // Plan label
        cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
        cr.set_font_size(9.5 * unit);
        cr.set_source_rgba(0.549, 0.580, 0.659, alpha);
        cr.move_to(cx + 11.0 * unit, cy + 35.0 * unit);
        let _ = cr.show_text(&trunc(&snap.plan_label, max_plan_chars));

        // Big percentage + suffix ("left" / "low" / "n/a")
        let pct_str = if is_unavail {
            "--".to_string()
        } else {
            format!("{}%", snap.remaining_pct)
        };
        cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        cr.set_font_size(23.0 * unit);
        if is_low {
            cr.set_source_rgba(0.980, 0.745, 0.141, alpha); // #FABE24
        } else if is_unavail {
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.35 * alpha);
        } else {
            cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
        }
        cr.move_to(cx + 11.0 * unit, cy + 63.0 * unit);
        let _ = cr.show_text(&pct_str);
        let pw = cr.text_extents(&pct_str).map(|e| e.x_advance()).unwrap_or(44.0 * unit);

        let suffix = if is_unavail {
            "n/a"
        } else if is_low {
            "low"
        } else {
            "left"
        };
        cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
        cr.set_font_size(10.5 * unit);
        cr.set_source_rgba(c1.0, c1.1, c1.2, (if is_unavail { 0.4 } else { 0.95 }) * alpha);
        cr.move_to(cx + 14.0 * unit + pw, cy + 63.0 * unit);
        let _ = cr.show_text(suffix);

        // Progress bar scaled to card width
        let bx = cx + 11.0 * unit;
        let by = cy + 74.0 * unit;
        let bw = (cw_raw - 24.0) * unit;
        rrect(cr, bx, by, bw, 5.0 * unit, 2.5 * unit);
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.10 * alpha);
        let _ = cr.fill();
        if !is_unavail && snap.remaining_pct > 0 {
            let fw = (bw * (snap.remaining_pct as f64 / 100.0)).max(5.0 * unit);
            let bg = LinearGradient::new(bx, by, bx + fw, by);
            bg.add_color_stop_rgba(0.0, c1.0, c1.1, c1.2, alpha);
            bg.add_color_stop_rgba(1.0, c2.0, c2.1, c2.2, alpha);
            rrect(cr, bx, by, fw, 5.0 * unit, 2.5 * unit);
            let _ = cr.set_source(&bg);
            let _ = cr.fill();
        }

        // Reset countdown
        cr.set_font_size(9.5 * unit);
        if is_low {
            cr.set_source_rgba(0.980, 0.745, 0.141, 0.82 * alpha);
        } else {
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.45 * alpha);
        }
        cr.move_to(cx + 11.0 * unit, cy + 96.0 * unit);
        let _ = cr.show_text(&trunc(&snap.reset_label(), max_reset_chars));
    }
}

fn provider_gradient(id: &str, is_low: bool) -> ((f64, f64, f64), (f64, f64, f64)) {
    if is_low {
        return ((0.957, 0.620, 0.043), (0.933, 0.267, 0.267)); // #F49E0B -> #EE4444
    }
    if id.starts_with("antigravity") {
        ((0.216, 0.741, 0.969), (0.388, 0.400, 0.945)) // #37BDF7 -> #6366F1
    } else if id.starts_with("codex") {
        ((0.063, 0.725, 0.506), (0.176, 0.831, 0.745)) // #10B981 -> #2DD4BE
    } else if id.starts_with("claude") {
        ((0.957, 0.620, 0.043), (0.933, 0.267, 0.267))
    } else {
        ((0.659, 0.333, 0.965), (0.216, 0.741, 0.969))
    }
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
