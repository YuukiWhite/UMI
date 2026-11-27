use gtk4::cairo::Context;
use gtk4::glib::translate::{FromGlibPtrFull, ToGlibPtr};
use gtk4::pango::{self, Alignment, EllipsizeMode, FontDescription, Layout, Weight, WrapMode};
use std::ffi::c_void;

// `libpangocairo-1.0` is already linked dynamically via `gtk4`.
// Using PangoCairo instead of Cairo's toy `show_text` gives full Fontconfig
// fallback for CJK (Japanese/Chinese/Korean), Arabic, Thai, Cyrillic, Emoji, etc.
#[link(name = "pangocairo-1.0")]
extern "C" {
    fn pango_cairo_create_layout(cr: *mut c_void) -> *mut pango::ffi::PangoLayout;
    fn pango_cairo_show_layout(cr: *mut c_void, layout: *mut pango::ffi::PangoLayout);
}

const FONT_STACK: &str =
    "Inter, Noto Sans CJK JP, Noto Sans CJK SC, Noto Sans CJK KR, Noto Sans Arabic, Noto Sans, Sans";

fn create_layout(cr: &Context, text: &str, size_px: f64, bold: bool) -> Layout {
    let raw_cr = cr.to_raw_none() as *mut c_void;
    let layout = unsafe {
        let raw_layout = pango_cairo_create_layout(raw_cr);
        Layout::from_glib_full(raw_layout)
    };
    let mut desc = FontDescription::from_string(FONT_STACK);
    // set_absolute_size takes Pango units in device pixels (matching Cairo set_font_size).
    desc.set_absolute_size((size_px.max(1.0) * pango::SCALE as f64).round());
    desc.set_weight(if bold { Weight::Bold } else { Weight::Normal });
    layout.set_font_description(Some(&desc));
    layout.set_text(text);
    layout
}

fn show_layout(cr: &Context, layout: &Layout) {
    let raw_cr = cr.to_raw_none() as *mut c_void;
    unsafe {
        pango_cairo_show_layout(raw_cr, layout.to_glib_none().0);
    }
}

/// Draw single-line text at `(x, top_y)` with optional `max_width` ellipsizing.
#[allow(dead_code, clippy::too_many_arguments)]
pub fn draw_text_line(
    cr: &Context,
    text: &str,
    x: f64,
    top_y: f64,
    max_width: Option<f64>,
    size_px: f64,
    bold: bool,
    rgba: (f64, f64, f64, f64),
) {
    if text.is_empty() || rgba.3 <= 0.005 {
        return;
    }
    let layout = create_layout(cr, text, size_px, bold);
    if let Some(mw) = max_width {
        layout.set_width((mw.max(4.0) * pango::SCALE as f64) as i32);
        layout.set_ellipsize(EllipsizeMode::End);
        layout.set_single_paragraph_mode(true);
    }
    let _ = cr.save();
    cr.set_source_rgba(rgba.0, rgba.1, rgba.2, rgba.3);
    cr.move_to(x, top_y);
    show_layout(cr, &layout);
    let _ = cr.restore();
}

/// Draw single-line text anchored by Cairo-style baseline `(x, baseline_y)` so it drops cleanly
/// into existing Cairo coordinates while supporting CJK / international scripts.
#[allow(clippy::too_many_arguments)]
pub fn draw_text_baseline(
    cr: &Context,
    text: &str,
    x: f64,
    baseline_y: f64,
    max_width: Option<f64>,
    size_px: f64,
    bold: bool,
    rgba: (f64, f64, f64, f64),
) {
    if text.is_empty() || rgba.3 <= 0.005 {
        return;
    }
    let layout = create_layout(cr, text, size_px, bold);
    if let Some(mw) = max_width {
        layout.set_width((mw.max(4.0) * pango::SCALE as f64) as i32);
        layout.set_ellipsize(EllipsizeMode::End);
        layout.set_single_paragraph_mode(true);
    }
    let baseline_px = layout.baseline() as f64 / pango::SCALE as f64;
    let _ = cr.save();
    cr.set_source_rgba(rgba.0, rgba.1, rgba.2, rgba.3);
    cr.move_to(x, baseline_y - baseline_px);
    show_layout(cr, &layout);
    let _ = cr.restore();
}

/// Draw multi-line wrapped text (WordChar wrap so CJK without spaces wraps cleanly)
/// centered vertically around `center_y` (great for Apple-Music-style gliding lyrics!).
#[allow(clippy::too_many_arguments)]
pub fn draw_text_centered_y(
    cr: &Context,
    text: &str,
    x: f64,
    center_y: f64,
    max_width: f64,
    max_lines: i32,
    size_px: f64,
    bold: bool,
    rgba: (f64, f64, f64, f64),
) {
    if text.is_empty() || rgba.3 <= 0.005 {
        return;
    }
    let layout = create_layout(cr, text, size_px, bold);
    layout.set_width((max_width.max(8.0) * pango::SCALE as f64) as i32);
    layout.set_wrap(WrapMode::WordChar);
    layout.set_ellipsize(EllipsizeMode::End);
    layout.set_height(-max_lines.max(1));
    layout.set_alignment(Alignment::Left);
    layout.set_spacing((size_px * 0.14 * pango::SCALE as f64) as i32);

    let (_, h_px) = layout.pixel_size();
    let _ = cr.save();
    cr.set_source_rgba(rgba.0, rgba.1, rgba.2, rgba.3);
    cr.move_to(x, center_y - h_px as f64 * 0.5);
    show_layout(cr, &layout);
    let _ = cr.restore();
}

/// Draw wrapped multi-line text top-aligned inside a box `(x, top_y, max_width)` up to `max_lines`.
#[allow(clippy::too_many_arguments)]
pub fn draw_text_wrapped(
    cr: &Context,
    text: &str,
    x: f64,
    top_y: f64,
    max_width: f64,
    max_lines: i32,
    size_px: f64,
    bold: bool,
    rgba: (f64, f64, f64, f64),
) {
    if text.is_empty() || rgba.3 <= 0.005 {
        return;
    }
    let layout = create_layout(cr, text, size_px, bold);
    layout.set_width((max_width.max(8.0) * pango::SCALE as f64) as i32);
    layout.set_wrap(WrapMode::WordChar);
    layout.set_ellipsize(EllipsizeMode::End);
    layout.set_height(-max_lines.max(1));
    layout.set_spacing((size_px * 0.22 * pango::SCALE as f64) as i32);

    let _ = cr.save();
    cr.set_source_rgba(rgba.0, rgba.1, rgba.2, rgba.3);
    cr.move_to(x, top_y);
    show_layout(cr, &layout);
    let _ = cr.restore();
}

/// Measure rendered pixel width of a single line (for strikethrough or centering).
pub fn measure_text_width(cr: &Context, text: &str, size_px: f64, bold: bool) -> f64 {
    if text.is_empty() {
        return 0.0;
    }
    let layout = create_layout(cr, text, size_px, bold);
    layout.pixel_size().0 as f64
}
