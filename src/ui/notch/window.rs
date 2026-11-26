use crate::ai::{AiRequest, AiResponse};
use crate::config::{self, AiConfig, NotchPageId, UmiConfig};
use crate::domain::pomodoro::PomodoroEngine;
use crate::domain::prayer_calc::{compute_today_schedule, PrayerSchedule};
use crate::lyrics::active_line_index;
use crate::quota::QuotaSnapshot;
use crate::state::machine::{NotchState, StateMachine};
use crate::state::spring::NotchSpring;
use crate::state::{AppEvent, ClipboardEntry, LyricsData, MediaCommand, MediaSnapshot};
use crate::storage::{LocalDb, ShelfItem, TaskItem, WaterSummary};
use crate::ui::notch::phase3::{
    focus_hit_zones, shelf_hit_zones, todo_hit_zones, water_hit_zones, water_settings_hit_zones,
    Phase3Data,
};
use crate::ui::notch::phase4::{
    ai_answer_hit_zones, ai_limits_hit_zones, ai_prompt_hit_zones, Phase4Data,
};
use crate::ui::notch::renderer::{accent_aura_stops, draw_notch, LiveData, RenderFrame};
use crate::ui::settings::build_settings_window;
use crate::ui::tray::TrayFlags;
use chrono::{Local, Timelike};
use gtk4::cairo::{Format, ImageSurface, RectangleInt, Region};
use gtk4::gdk;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, CssProvider, DragSource, DrawingArea, DropTarget, Entry,
    EventControllerKey, EventControllerMotion, EventControllerScroll, EventControllerScrollFlags,
    GestureClick, Overlay,
};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

fn canvas_dimensions(scale: f64) -> (i32, i32) {
    let w = ((520.0 + 96.0) * scale).ceil() as i32;
    let h = ((140.0 + 48.0) * scale).ceil() as i32;
    (w, h)
}

struct NotchRuntime {
    config: UmiConfig,
    db: Option<LocalDb>,
    sm: StateMachine,
    spring: NotchSpring,
    start_instant: Instant,
    last_frame_time_us: Option<i64>,
    hover_active: bool,
    left_arrow_hovered: bool,
    right_arrow_hovered: bool,
    hovered_dot_idx: Option<usize>,
    dot_hover_anim: [f64; 12],
    dot_hover_vel: [f64; 12],
    dot_active_anim: [f64; 12],
    dot_active_vel: [f64; 12],
    aura_rgb: [(f64, f64, f64); 3],
    eye_tint_rgb: (f64, f64, f64),
    float_mix: f64,
    ear_mix: f64,
    glass_mix: f64,
    pending_expand: Option<glib::SourceId>,
    pending_collapse: Option<glib::SourceId>,
    pending_peek_dismiss: Option<glib::SourceId>,
    last_input_rect: Option<(i32, i32, i32, i32)>,
    last_canvas_w: i32,
    status_text: Arc<RwLock<String>>,
    tray_flags: TrayFlags,
    // Phase 2 live system state
    battery_pct: u8,
    battery_charging: bool,
    media: MediaSnapshot,
    loaded_art_path: Option<PathBuf>,
    album_art: Option<ImageSurface>,
    lyrics_track_key: String,
    lyrics: LyricsData,
    lyrics_scroll_idx: f64,
    lyrics_scroll_vel: f64,
    clipboard: Vec<ClipboardEntry>,
    copied_flash_idx: Option<usize>,
    media_cmd_tx: async_channel::Sender<MediaCommand>,
    clip_copy_tx: async_channel::Sender<String>,
    // Phase 3 productivity & utility state
    pomodoro: PomodoroEngine,
    prayer: PrayerSchedule,
    last_prayer_minute: i32,
    water: WaterSummary,
    shelf: Vec<ShelfItem>,
    shelf_drag_over: bool,
    tasks: Vec<TaskItem>,
    todo_edit_mode: bool,
    todo_scroll: usize,
    todo_entry_open: bool,
    // Phase 4 Ask UMI AI, Smart OCR & AI Limits state
    ai_req_tx: async_channel::Sender<AiRequest>,
    quota_refresh_tx: async_channel::Sender<()>,
    ai_translate_mode: bool,
    ai_loading: bool,
    ai_last_request: Option<AiRequest>,
    ai_response: Option<AiResponse>,
    ai_copied_flash: bool,
    quotas: Vec<QuotaSnapshot>,
    // Phase 5 Desktop Settings window handle
    settings_win: Option<glib::WeakRef<ApplicationWindow>>,
}

impl NotchRuntime {
    fn sync_targets(&mut self, window: &ApplicationWindow, ai_entry: &Entry) {
        let tier = self.sm.current.tier(self.config.general.minimal_notch_mode);
        let geom = tier.geometry();
        let expanded_mix = if self.sm.current.is_expanded_or_wide() {
            1.0
        } else {
            0.0
        };
        self.spring.set_targets(
            geom.width_dp,
            geom.height_dp,
            geom.bottom_radius_dp,
            geom.ear_radius_dp,
            expanded_mix,
            self.config.appearance.spring_speed,
        );

        let show_ai_entry = self.sm.current == NotchState::AiPrompt;
        if WidgetExt::is_visible(ai_entry) != show_ai_entry {
            ai_entry.set_visible(show_ai_entry);
            if show_ai_entry {
                ai_entry.grab_focus();
            }
        }
        ai_entry.set_placeholder_text(Some(if self.ai_translate_mode {
            "Type text to translate into Indonesian · Enter to send"
        } else {
            "Ask UMI anything, translate, or type 100usd to idr= ..."
        }));

        let kb_mode = if self.sm.current.is_expanded_or_wide()
            || self.sm.current.requires_keyboard()
            || self.todo_entry_open
            || show_ai_entry
        {
            KeyboardMode::OnDemand
        } else {
            KeyboardMode::None
        };
        window.set_keyboard_mode(kb_mode);

        if let Some(child) = window.child() {
            child.queue_draw();
        }

        if let Ok(mut guard) = self.status_text.write() {
            *guard = format!(
                "state={} page={} scale={:.2} bat={}% media={} tasks={} quotas={}",
                self.sm.current.status_name(),
                self.sm.current_page().title(),
                self.config.appearance.scale,
                self.battery_pct,
                if self.media.playing { "playing" } else { "idle" },
                self.tasks.len(),
                self.quotas.len(),
            );
        }
    }

    fn submit_ai_request(&mut self, req: AiRequest, window: &ApplicationWindow, ai_entry: &Entry) {
        self.ai_loading = true;
        self.ai_last_request = Some(req.clone());
        self.sm.jump_to_page(NotchPageId::AskAi);
        self.sm.current = NotchState::AiAnswer;
        self.sm.interactive_lock = false;
        let _ = self.ai_req_tx.try_send(req);
        self.sync_targets(window, ai_entry);
    }

    fn cancel_timers(&mut self) {
        if let Some(id) = self.pending_expand.take() {
            id.remove();
        }
        if let Some(id) = self.pending_collapse.take() {
            id.remove();
        }
    }

    fn refresh_album_art(&mut self) {
        if self.media.art_path == self.loaded_art_path {
            return;
        }
        self.loaded_art_path = self.media.art_path.clone();
        self.album_art = self
            .loaded_art_path
            .as_deref()
            .and_then(load_image_surface);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_notch_window(
    app: &Application,
    cfg: UmiConfig,
    db: Option<LocalDb>,
    event_rx: async_channel::Receiver<AppEvent>,
    event_tx: async_channel::Sender<AppEvent>,
    media_cmd_tx: async_channel::Sender<MediaCommand>,
    clip_copy_tx: async_channel::Sender<String>,
    ai_req_tx: async_channel::Sender<AiRequest>,
    quota_refresh_tx: async_channel::Sender<()>,
    ai_cfg: Arc<RwLock<AiConfig>>,
    rt_handle: tokio::runtime::Handle,
    status_text: Arc<RwLock<String>>,
    tray_flags: TrayFlags,
) -> ApplicationWindow {
    load_css();

    let scale = cfg.appearance.scale;
    let unit = scale / 1.3;
    let (canvas_w, canvas_h) = canvas_dimensions(scale);
    let window = ApplicationWindow::builder()
        .application(app)
        .title("UMI Notch")
        .decorated(false)
        .resizable(false)
        .default_width(canvas_w)
        .default_height(canvas_h)
        .build();
    window.add_css_class("umi-notch-window");

    window.init_layer_shell();
    window.set_namespace("umi-notch");
    window.set_layer(if cfg.general.hide_in_fullscreen {
        Layer::Top
    } else {
        Layer::Overlay
    });
    apply_target_monitor(&window, cfg.appearance.display_monitor.as_deref());
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Left, false);
    window.set_anchor(Edge::Right, false);
    window.set_anchor(Edge::Bottom, false);
    window.set_exclusive_zone(-1);
    window.set_keyboard_mode(KeyboardMode::None);

    let water_init = db
        .as_ref()
        .and_then(|d| d.today_water_summary().ok())
        .unwrap_or_default();
    let shelf_init = db
        .as_ref()
        .and_then(|d| d.list_shelf_items().ok())
        .unwrap_or_default();
    let tasks_init = db
        .as_ref()
        .and_then(|d| d.list_tasks().ok())
        .unwrap_or_default();

    let sm = StateMachine::new(cfg.pages.active.clone(), cfg.general.idle_mascot_eyes);
    let init_geom = sm.current.tier(cfg.general.minimal_notch_mode).geometry();
    let spring = NotchSpring::new(
        init_geom.width_dp,
        init_geom.height_dp,
        init_geom.bottom_radius_dp,
        init_geom.ear_radius_dp,
    );
    let pomodoro = PomodoroEngine::new(cfg.focus.clone());
    let prayer = compute_today_schedule(&cfg.prayer);
    let now_min = (Local::now().hour() * 60 + Local::now().minute()) as i32;

    let overlay = Overlay::new();
    let drawing = DrawingArea::new();
    drawing.set_content_width(canvas_w);
    drawing.set_content_height(canvas_h);
    drawing.set_hexpand(true);
    drawing.set_vexpand(true);
    overlay.set_child(Some(&drawing));

    // Inline GTK4 Entry for "+ Add Task" inside Notch 13 To-do
    let todo_entry = Entry::builder()
        .placeholder_text("New task title · Enter to save, Esc to cancel")
        .visible(false)
        .halign(Align::Center)
        .valign(Align::Start)
        .width_request((230.0 * scale) as i32)
        .margin_top((86.0 * scale) as i32)
        .build();
    todo_entry.add_css_class("umi-inline-entry");
    overlay.add_overlay(&todo_entry);

    // Transparent live GTK4 Entry positioned inside Notch 15 Ask UMI AI glowing prompt box
    let ai_entry = Entry::builder()
        .placeholder_text("Ask UMI anything, translate, or type 100usd to idr= ...")
        .visible(false)
        .has_frame(false)
        .halign(Align::Center)
        .valign(Align::Start)
        .width_request((392.0 * unit) as i32)
        .margin_top((48.0 * unit) as i32)
        .margin_end((16.0 * unit) as i32)
        .build();
    ai_entry.add_css_class("umi-ai-entry");
    overlay.add_overlay(&ai_entry);

    let mut init_dot_active = [0.0_f64; 12];
    if sm.active_page_idx < 12 {
        init_dot_active[sm.active_page_idx] = 1.0;
    }

    let init_aura_rgb = accent_aura_stops(&cfg.appearance.accent_aura);
    let init_eye_tint_rgb = cfg.appearance.eye_tint.rgb();
    let init_float_mix = if cfg.appearance.style == config::NotchStyle::Floating { 1.0 } else { 0.0 };
    let init_ear_mix = if cfg.appearance.smooth_concave_fillets
        && cfg.appearance.style != config::NotchStyle::Floating
    {
        1.0
    } else {
        0.0
    };
    let init_glass_mix = if cfg.appearance.style == config::NotchStyle::Glass { 1.0 } else { 0.0 };

    let rt = Rc::new(RefCell::new(NotchRuntime {
        config: cfg,
        db,
        sm,
        spring,
        start_instant: Instant::now(),
        last_frame_time_us: None,
        hover_active: false,
        left_arrow_hovered: false,
        right_arrow_hovered: false,
        hovered_dot_idx: None,
        dot_hover_anim: [0.0; 12],
        dot_hover_vel: [0.0; 12],
        dot_active_anim: init_dot_active,
        dot_active_vel: [0.0; 12],
        aura_rgb: init_aura_rgb,
        eye_tint_rgb: init_eye_tint_rgb,
        float_mix: init_float_mix,
        ear_mix: init_ear_mix,
        glass_mix: init_glass_mix,
        pending_expand: None,
        pending_collapse: None,
        pending_peek_dismiss: None,
        last_input_rect: None,
        last_canvas_w: 0,
        status_text,
        tray_flags,
        battery_pct: 84,
        battery_charging: false,
        media: MediaSnapshot::default(),
        loaded_art_path: None,
        album_art: None,
        lyrics_track_key: String::new(),
        lyrics: LyricsData::Idle,
        lyrics_scroll_idx: 0.0,
        lyrics_scroll_vel: 0.0,
        clipboard: Vec::new(),
        copied_flash_idx: None,
        media_cmd_tx,
        clip_copy_tx,
        pomodoro,
        prayer,
        last_prayer_minute: now_min,
        water: water_init,
        shelf: shelf_init,
        shelf_drag_over: false,
        tasks: tasks_init,
        todo_edit_mode: false,
        todo_scroll: 0,
        todo_entry_open: false,
        ai_req_tx,
        quota_refresh_tx,
        ai_translate_mode: false,
        ai_loading: false,
        ai_last_request: None,
        ai_response: None,
        ai_copied_flash: false,
        quotas: Vec::new(),
        settings_win: None,
    }));
    rt.borrow_mut().sync_targets(&window, &ai_entry);

    // Inline To-do entry Enter handler
    {
        let rt = Rc::clone(&rt);
        let win_weak = window.downgrade();
        let ai_weak = ai_entry.downgrade();
        todo_entry.connect_activate(move |entry| {
            let text = entry.text().to_string();
            entry.set_text("");
            entry.set_visible(false);
            let mut st = rt.borrow_mut();
            st.todo_entry_open = false;
            st.sm.interactive_lock = false;
            if let Some(ref db) = st.db {
                if let Ok(list) = db.add_task(&text) {
                    st.tasks = list;
                    st.todo_scroll = 0;
                }
            }
            if let (Some(w), Some(ae)) = (win_weak.upgrade(), ai_weak.upgrade()) {
                st.sync_targets(&w, &ae);
            }
        });
    }

    // Ask UMI AI prompt Enter handler + lock while typing
    {
        let rt = Rc::clone(&rt);
        let win_weak = window.downgrade();
        ai_entry.connect_activate(move |entry| {
            let prompt = entry.text().to_string();
            if prompt.trim().is_empty() {
                return;
            }
            entry.set_text("");
            let mut st = rt.borrow_mut();
            let req = AiRequest::Ask {
                prompt,
                translate_mode: st.ai_translate_mode,
            };
            if let Some(w) = win_weak.upgrade() {
                st.submit_ai_request(req, &w, entry);
            }
        });
    }
    {
        let rt = Rc::clone(&rt);
        ai_entry.connect_changed(move |entry| {
            let has_text = !entry.text().is_empty();
            rt.borrow_mut().sm.interactive_lock = has_text;
        });
    }

    // Ask UMI AI prompt keyboard shortcuts (Tab / Left / Right / Esc) are handled in Capture phase on `window` below.

    // Cairo draw callback
    {
        let rt = Rc::clone(&rt);
        let ai_weak = ai_entry.downgrade();
        drawing.set_draw_func(move |_area, cr, w, h| {
            let state = rt.borrow();
            let mut frame = RenderFrame::from_state(
                w as f64,
                h as f64,
                &state.spring,
                state.config.appearance.scale,
                state.start_instant.elapsed().as_secs_f64(),
                state.left_arrow_hovered,
                state.right_arrow_hovered,
            );
            frame.hovered_dot_idx = state.hovered_dot_idx;
            frame.dot_hover_anim = state.dot_hover_anim;
            frame.dot_active_anim = state.dot_active_anim;
            frame.has_dot_anim = true;
            frame.aura_rgb = state.aura_rgb;
            frame.eye_tint_rgb = state.eye_tint_rgb;
            frame.has_color_anim = true;
            frame.float_mix = state.float_mix;
            frame.ear_mix = state.ear_mix;
            frame.glass_mix = state.glass_mix;
            frame.has_style_anim = true;
            let ai_vis = ai_weak
                .upgrade()
                .map(|e| WidgetExt::is_visible(&e))
                .unwrap_or(false);
            let live = LiveData {
                battery_pct: state.battery_pct,
                media: &state.media,
                album_art: state.album_art.as_ref(),
                lyrics: &state.lyrics,
                lyrics_scroll_idx: state.lyrics_scroll_idx,
                clipboard: &state.clipboard,
                copied_flash_idx: state.copied_flash_idx,
                phase3: Phase3Data {
                    pomodoro: &state.pomodoro,
                    prayer: &state.prayer,
                    water: &state.water,
                    shelf: &state.shelf,
                    shelf_drag_over: state.shelf_drag_over,
                    tasks: &state.tasks,
                    todo_edit_mode: state.todo_edit_mode,
                    todo_scroll: state.todo_scroll,
                    battery_pct: state.battery_pct,
                    battery_charging: state.battery_charging,
                },
                phase4: Phase4Data {
                    translate_mode: state.ai_translate_mode,
                    ai_loading: state.ai_loading,
                    ai_response: state.ai_response.as_ref(),
                    ai_copied_flash: state.ai_copied_flash,
                    quotas: &state.quotas,
                },
                ai_entry_visible: ai_vis,
            };
            draw_notch(cr, &frame, &state.sm, &state.config, &live);
        });
    }

    // VSync-locked animation tick + Pomodoro + Prayer + Subpixel Lyrics Glide
    {
        let rt = Rc::clone(&rt);
        let win_weak = window.downgrade();
        let ai_weak = ai_entry.downgrade();
        drawing.add_tick_callback(move |area, clock| {
            let mut st = rt.borrow_mut();
            let now_us = clock.frame_time();
            let dt = match st.last_frame_time_us {
                Some(prev) if now_us > prev => ((now_us - prev) as f64) / 1_000_000.0,
                _ => 1.0 / 60.0,
            }
            .clamp(0.001, 0.05);
            st.last_frame_time_us = Some(now_us);

            let speed = st.config.appearance.spring_speed;
            let was_active = st.spring.is_active();
            let spring_moving = st.spring.step(dt, speed);

            if st.pomodoro.running {
                if let Some(msg) = st.pomodoro.tick(dt) {
                    st.sm.enqueue_peek(NotchState::Notification {
                        app: "Focus Timer".into(),
                        sender: "Pomodoro".into(),
                        body: msg,
                    });
                    if st.sm.current.is_peek() {
                        schedule_peek_dismiss(&rt, &mut st, &win_weak, &ai_weak);
                    }
                    if let (Some(w), Some(ae)) = (win_weak.upgrade(), ai_weak.upgrade()) {
                        st.sync_targets(&w, &ae);
                    }
                }
            }

            let now = Local::now();
            let now_min = (now.hour() * 60 + now.minute()) as i32;
            if now_min != st.last_prayer_minute {
                st.last_prayer_minute = now_min;
                let prev_entries = st.prayer.entries.clone();
                st.prayer = compute_today_schedule(&st.config.prayer);
                if let Some(hit) = prev_entries.iter().find(|e| e.minutes_from_midnight == now_min) {
                    if st.config.prayer.auto_pause_music && st.media.playing {
                        let _ = st.media_cmd_tx.try_send(MediaCommand::PlayPause);
                    }
                    let city = st.config.prayer.city.clone();
                    st.sm.enqueue_peek(NotchState::Notification {
                        app: "Prayer".into(),
                        sender: format!("Waktu {} ({city})", hit.name),
                        body: "Media dijeda otomatis · Selamat menunaikan ibadah.".into(),
                    });
                    if st.sm.current.is_peek() {
                        schedule_peek_dismiss(&rt, &mut st, &win_weak, &ai_weak);
                    }
                    if let (Some(w), Some(ae)) = (win_weak.upgrade(), ai_weak.upgrade()) {
                        st.sync_targets(&w, &ae);
                    }
                }
            }

            let mut lyrics_moving = false;
            if let LyricsData::Synced(ref lines) = st.lyrics {
                if !lines.is_empty() {
                    let pos_ms = (st.media.live_position_us() / 1000).max(0) as u64;
                    let target = active_line_index(lines, pos_ms).unwrap_or(0) as f64;
                    let diff = target - st.lyrics_scroll_idx;
                    if diff.abs() > 6.0 {
                        st.lyrics_scroll_idx = target;
                        st.lyrics_scroll_vel = 0.0;
                        lyrics_moving = true;
                    } else if diff.abs() > 0.001 || st.lyrics_scroll_vel.abs() > 0.005 {
                        let sub_dt = dt / 4.0;
                        for _ in 0..4 {
                            let disp = st.lyrics_scroll_idx - target;
                            let acc = -120.0 * disp - 21.9 * st.lyrics_scroll_vel;
                            st.lyrics_scroll_vel += acc * sub_dt;
                            st.lyrics_scroll_idx += st.lyrics_scroll_vel * sub_dt;
                        }
                        lyrics_moving = true;
                    } else {
                        st.lyrics_scroll_idx = target;
                        st.lyrics_scroll_vel = 0.0;
                    }
                }
            }

            // Smooth VSync spring animation for bottom pager dots (bouncy hover pop + silky pill morph)
            let mut dots_moving = false;
            let dot_count = st.sm.pages.len().min(12);
            let active_idx = st.sm.active_page_idx;
            let hovered_idx = st.hovered_dot_idx;
            let sub_dt = dt * 0.5;
            for i in 0..dot_count {
                let h_target = if hovered_idx == Some(i) { 1.0 } else { 0.0 };
                let a_target = if active_idx == i { 1.0 } else { 0.0 };

                if (st.dot_hover_anim[i] - h_target).abs() > 0.002 || st.dot_hover_vel[i].abs() > 0.01 {
                    for _ in 0..2 {
                        let disp = st.dot_hover_anim[i] - h_target;
                        let acc = -340.0 * disp - 26.0 * st.dot_hover_vel[i];
                        st.dot_hover_vel[i] += acc * sub_dt;
                        st.dot_hover_anim[i] += st.dot_hover_vel[i] * sub_dt;
                    }
                    dots_moving = true;
                } else {
                    st.dot_hover_anim[i] = h_target;
                    st.dot_hover_vel[i] = 0.0;
                }

                if (st.dot_active_anim[i] - a_target).abs() > 0.002 || st.dot_active_vel[i].abs() > 0.01 {
                    for _ in 0..2 {
                        let disp = st.dot_active_anim[i] - a_target;
                        let acc = -240.0 * disp - 30.0 * st.dot_active_vel[i];
                        st.dot_active_vel[i] += acc * sub_dt;
                        st.dot_active_anim[i] += st.dot_active_vel[i] * sub_dt;
                    }
                    dots_moving = true;
                } else {
                    st.dot_active_anim[i] = a_target;
                    st.dot_active_vel[i] = 0.0;
                }
            }

            // Snappy VSync color & notch-style morph (~200-260ms silky transition)
            let mut color_moving = false;
            let ck = 1.0 - (-12.0 * dt).exp();
            let target_aura = accent_aura_stops(&st.config.appearance.accent_aura);
            for i in 0..3 {
                let (tr, tg, tb) = target_aura[i];
                let (cr, cg, cb) = st.aura_rgb[i];
                if (tr - cr).abs() > 0.0015 || (tg - cg).abs() > 0.0015 || (tb - cb).abs() > 0.0015 {
                    st.aura_rgb[i] = (cr + (tr - cr) * ck, cg + (tg - cg) * ck, cb + (tb - cb) * ck);
                    color_moving = true;
                } else {
                    st.aura_rgb[i] = (tr, tg, tb);
                }
            }
            let (etr, etg, etb) = st.config.appearance.eye_tint.rgb();
            let (ecr, ecg, ecb) = st.eye_tint_rgb;
            if (etr - ecr).abs() > 0.0015 || (etg - ecg).abs() > 0.0015 || (etb - ecb).abs() > 0.0015 {
                st.eye_tint_rgb = (ecr + (etr - ecr) * ck, ecg + (etg - ecg) * ck, ecb + (etb - ecb) * ck);
                color_moving = true;
            } else {
                st.eye_tint_rgb = (etr, etg, etb);
            }

            let sk = 1.0 - (-16.0 * dt).exp();
            let target_float = if st.config.appearance.style == config::NotchStyle::Floating { 1.0 } else { 0.0 };
            let target_ear = if st.config.appearance.smooth_concave_fillets
                && st.config.appearance.style != config::NotchStyle::Floating
            {
                1.0
            } else {
                0.0
            };
            let target_glass = if st.config.appearance.style == config::NotchStyle::Glass { 1.0 } else { 0.0 };
            let mut style_moving = false;
            let mut step_val = |cur: &mut f64, tgt: f64| {
                if (*cur - tgt).abs() > 0.002 {
                    *cur += (tgt - *cur) * sk;
                    style_moving = true;
                } else {
                    *cur = tgt;
                }
            };
            let st_ref = &mut *st;
            step_val(&mut st_ref.float_mix, target_float);
            step_val(&mut st_ref.ear_mix, target_ear);
            step_val(&mut st_ref.glass_mix, target_glass);

            let cur_w = area.width();
            if cur_w >= 100 {
                if let Some(win) = win_weak.upgrade() {
                    let resized = cur_w != st.last_canvas_w;
                    let expanding = st.spring.width.target > st.spring.width.value + 1.0;
                    if resized || st.last_input_rect.is_none() || expanding || (was_active && !spring_moving) {
                        st.last_canvas_w = cur_w;
                        update_input_region(&win, &mut st, cur_w as f64, expanding);
                    }
                }
            }

            let elapsed = st.start_instant.elapsed().as_secs_f64();
            let in_blink = (elapsed % 4.4) > 4.20;
            let needs_live_redraw = lyrics_moving
                || dots_moving
                || color_moving
                || style_moving
                || st.pomodoro.running
                || st.ai_loading
                || (st.media.playing
                    && (matches!(st.sm.current, NotchState::Closed | NotchState::Music | NotchState::Lyrics)
                        || st.sm.current.is_expanded_or_wide()))
                || st.sm.current.is_expanded_or_wide();

            if spring_moving || was_active || in_blink || needs_live_redraw {
                area.queue_draw();
            }

            glib::ControlFlow::Continue
        });
    }

    // Wayland Drag-and-Drop IN (Shelf)
    let drop_target = DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);
    {
        let rt = Rc::clone(&rt);
        let win_weak = window.downgrade();
        let ai_weak = ai_entry.downgrade();
        drop_target.connect_enter(move |_dt, _x, _y| {
            let mut st = rt.borrow_mut();
            st.cancel_timers();
            st.shelf_drag_over = true;
            st.sm.jump_to_page(NotchPageId::Shelf);
            if let (Some(w), Some(ae)) = (win_weak.upgrade(), ai_weak.upgrade()) {
                st.sync_targets(&w, &ae);
            }
            gdk::DragAction::COPY
        });
    }
    {
        let rt = Rc::clone(&rt);
        let area_weak = drawing.downgrade();
        drop_target.connect_leave(move |_dt| {
            rt.borrow_mut().shelf_drag_over = false;
            if let Some(a) = area_weak.upgrade() {
                a.queue_draw();
            }
        });
    }
    {
        let rt = Rc::clone(&rt);
        let win_weak = window.downgrade();
        let ai_weak = ai_entry.downgrade();
        drop_target.connect_drop(move |_dt, value, _x, _y| {
            let mut st = rt.borrow_mut();
            st.shelf_drag_over = false;
            let Ok(list) = value.get::<gdk::FileList>() else {
                return false;
            };
            let mut added_any = false;
            for file in list.files() {
                let uri = file.uri().to_string();
                let name = file
                    .basename()
                    .and_then(|p| p.file_name().map(|s| s.to_string_lossy().to_string()))
                    .unwrap_or_else(|| "file".to_string());
                let size_bytes = file
                    .path()
                    .and_then(|p| std::fs::metadata(p).ok())
                    .map(|m| m.len())
                    .unwrap_or(0);
                if let Some(ref db) = st.db {
                    if let Ok(items) = db.add_shelf_item(&uri, &name, size_bytes) {
                        st.shelf = items;
                        added_any = true;
                    }
                } else {
                    st.shelf.insert(0, ShelfItem { uri, name, size_bytes });
                    added_any = true;
                }
            }
            if let (Some(w), Some(ae)) = (win_weak.upgrade(), ai_weak.upgrade()) {
                st.sync_targets(&w, &ae);
            }
            added_any
        });
    }
    drawing.add_controller(drop_target);

    // Wayland Drag-and-Drop OUT (Shelf)
    let drag_source = DragSource::new();
    drag_source.set_actions(gdk::DragAction::COPY);
    {
        let rt = Rc::clone(&rt);
        let area_weak = drawing.downgrade();
        drag_source.connect_prepare(move |_src, dx, dy| {
            let st = rt.borrow();
            if st.sm.current != NotchState::Shelf || st.shelf.is_empty() {
                return None;
            }
            let area = area_weak.upgrade()?;
            let frame = RenderFrame::from_state(
                area.width() as f64,
                area.height() as f64,
                &st.spring,
                st.config.appearance.scale,
                0.0,
                false,
                false,
            );
            let unit = frame.scale / 1.3;
            let target_w = 420.0 * frame.scale;
            let x = frame.notch_x + (frame.notch_w - target_w) * 0.5;
            let top_y = frame.top_y(st.config.appearance.style);
            let sz = shelf_hit_zones(x, top_y, unit, st.shelf.len());

            for (idx, rect) in sz.item_rects.iter().enumerate() {
                if rect.contains(dx, dy) {
                    if let Some(item) = st.shelf.get(idx) {
                        let file = gio::File::for_uri(&item.uri);
                        let flist = gdk::FileList::from_array(&[file]);
                        return Some(gdk::ContentProvider::for_value(&flist.to_value()));
                    }
                }
            }
            None
        });
    }
    drawing.add_controller(drag_source);

    // Hover motion controller attached to overlay so hovering ai_entry never triggers false leave
    let motion = EventControllerMotion::new();
    {
        let rt = Rc::clone(&rt);
        let win_weak = window.downgrade();
        let ai_weak = ai_entry.downgrade();
        let area_weak = drawing.downgrade();
        motion.connect_enter(move |_ctrl, _x, _y| {
            let mut st = rt.borrow_mut();
            st.hover_active = true;
            if let Some(id) = st.pending_collapse.take() {
                id.remove();
            }
            if st.sm.current.is_compact() {
                st.sm.expand();
                if let (Some(w), Some(ae)) = (win_weak.upgrade(), ai_weak.upgrade()) {
                    st.sync_targets(&w, &ae);
                }
                if let Some(a) = area_weak.upgrade() {
                    a.queue_draw();
                }
            }
        });
    }
    {
        let rt = Rc::clone(&rt);
        let area_weak = drawing.downgrade();
        motion.connect_motion(move |_ctrl, mx, my| {
            let mut st = rt.borrow_mut();
            if let Some(area) = area_weak.upgrade() {
                let frame = RenderFrame::from_state(
                    area.width() as f64,
                    area.height() as f64,
                    &st.spring,
                    st.config.appearance.scale,
                    0.0,
                    false,
                    false,
                );
                let top_y = frame.top_y(st.config.appearance.style);
                let unit = frame.scale / 1.3;
                let is_ai = matches!(st.sm.current, NotchState::AiPrompt | NotchState::AiAnswer);
                let x_off = if is_ai { 10.0 * unit } else { 0.0 };
                let (lx, ly) = frame.left_arrow_center();
                let (rx, ry) = frame.right_arrow_center();
                let hit_r = if is_ai {
                    frame.arrow_radius() * 0.88 + 5.0
                } else {
                    frame.arrow_radius() + 4.0
                };
                let prev_l = st.left_arrow_hovered;
                let prev_r = st.right_arrow_hovered;
                let prev_dot = st.hovered_dot_idx;
                let show_nav = st.sm.current.is_expanded_or_wide()
                    && !matches!(st.sm.current, NotchState::Lyrics | NotchState::InNotchSettings);
                st.left_arrow_hovered =
                    show_nav && ((mx - (lx - x_off)).hypot(my - (top_y + ly)) <= hit_r);
                st.right_arrow_hovered =
                    show_nav && ((mx - (rx + x_off)).hypot(my - (top_y + ry)) <= hit_r);
                st.hovered_dot_idx = if show_nav {
                    frame
                        .dot_hit_rects(top_y, st.sm.pages.len(), st.sm.active_page_idx)
                        .iter()
                        .position(|r| r.contains(mx, my))
                } else {
                    None
                };
                let over_nav =
                    st.left_arrow_hovered || st.right_arrow_hovered || st.hovered_dot_idx.is_some();
                area.set_cursor_from_name(if over_nav { Some("pointer") } else { None });
                if st.left_arrow_hovered != prev_l
                    || st.right_arrow_hovered != prev_r
                    || st.hovered_dot_idx != prev_dot
                {
                    area.queue_draw();
                }
            }
        });
    }
    {
        let rt = Rc::clone(&rt);
        let win_weak = window.downgrade();
        let ai_weak = ai_entry.downgrade();
        let area_weak = drawing.downgrade();
        motion.connect_leave(move |_ctrl| {
            let mut st = rt.borrow_mut();
            st.hover_active = false;
            st.left_arrow_hovered = false;
            st.right_arrow_hovered = false;
            st.hovered_dot_idx = None;
            if let Some(a) = area_weak.upgrade() {
                a.set_cursor_from_name(None);
            }
            if let Some(id) = st.pending_expand.take() {
                id.remove();
            }
            if st.sm.current.is_expanded_or_wide() && st.pending_collapse.is_none() {
                let rt_cb = Rc::clone(&rt);
                let win_cb = win_weak.clone();
                let ai_cb = ai_weak.clone();
                let area_cb = area_weak.clone();
                let id = glib::timeout_add_local_once(Duration::from_millis(70), move || {
                    let mut st = rt_cb.borrow_mut();
                    st.pending_collapse = None;
                    if !st.hover_active && !st.todo_entry_open && !st.ai_loading {
                        st.sm.collapse(false);
                        if st.sm.current.is_peek() {
                            schedule_peek_dismiss(&rt_cb, &mut st, &win_cb, &ai_cb);
                        }
                        if let (Some(w), Some(ae)) = (win_cb.upgrade(), ai_cb.upgrade()) {
                            st.sync_targets(&w, &ae);
                        }
                        if let Some(a) = area_cb.upgrade() {
                            a.queue_draw();
                        }
                    }
                });
                st.pending_collapse = Some(id);
            }
        });
    }
    overlay.add_controller(motion);

    // Click handler across all Phases 1–5 interactive modules
    let click = GestureClick::new();
    {
        let rt = Rc::clone(&rt);
        let win_weak = window.downgrade();
        let area_weak = drawing.downgrade();
        let entry_weak = todo_entry.downgrade();
        let ai_weak = ai_entry.downgrade();
        let event_tx_click = event_tx.clone();
        let ai_cfg_click = Arc::clone(&ai_cfg);
        let rt_handle_click = rt_handle.clone();
        click.connect_pressed(move |_g, _n, cx, cy| {
            let mut st = rt.borrow_mut();
            if st.sm.current.is_compact() {
                st.sm.toggle_compact_variant();
            } else if st.sm.current.is_peek() {
                if let Some(id) = st.pending_peek_dismiss.take() {
                    id.remove();
                }
                if matches!(st.sm.current, NotchState::QuotaLow { .. }) {
                    st.sm.jump_to_page(NotchPageId::AiLimits);
                } else {
                    st.sm.expand();
                }
            } else if let Some(area) = area_weak.upgrade() {
                let frame = RenderFrame::from_state(
                    area.width() as f64,
                    area.height() as f64,
                    &st.spring,
                    st.config.appearance.scale,
                    0.0,
                    false,
                    false,
                );
                let top_y = frame.top_y(st.config.appearance.style);
                let unit = frame.scale / 1.3;
                let target_w = 420.0 * frame.scale;
                let x = frame.notch_x + (frame.notch_w - target_w) * 0.5;
                let mut handled = false;

                // 1. Music / Lyrics
                if matches!(st.sm.current, NotchState::Music | NotchState::Lyrics) {
                    let wide = st.sm.current == NotchState::Lyrics;
                    let zones = frame.music_hit_zones(wide, top_y);
                    if zones.lyrics_btn.contains(cx, cy) {
                        st.sm.toggle_lyrics();
                        handled = true;
                    } else if zones.prev_btn.contains(cx, cy) {
                        let _ = st.media_cmd_tx.try_send(MediaCommand::Previous);
                        handled = true;
                    } else if zones.play_btn.contains(cx, cy) {
                        let _ = st.media_cmd_tx.try_send(MediaCommand::PlayPause);
                        st.media.playing = !st.media.playing;
                        handled = true;
                    } else if zones.next_btn.contains(cx, cy) {
                        let _ = st.media_cmd_tx.try_send(MediaCommand::Next);
                        handled = true;
                    } else if zones.progress_bar.contains(cx, cy) {
                        let ratio = ((cx - zones.progress_bar.x) / zones.progress_bar.w).clamp(0.0, 1.0);
                        let _ = st.media_cmd_tx.try_send(MediaCommand::SeekRatio(ratio));
                        handled = true;
                    } else if wide {
                        if let LyricsData::Synced(ref lines) = st.lyrics {
                            let pos_ms = (st.media.live_position_us() / 1000).max(0) as u64;
                            if let Some(idx) = active_line_index(lines, pos_ms) {
                                if zones.lyric_prev_line.map(|r| r.contains(cx, cy)).unwrap_or(false) {
                                    if let Some(target) = idx.checked_sub(1).and_then(|i| lines.get(i)) {
                                        let _ = st.media_cmd_tx.try_send(MediaCommand::SeekMs(target.time_ms));
                                        handled = true;
                                    }
                                } else if zones.lyric_next_line.map(|r| r.contains(cx, cy)).unwrap_or(false) {
                                    if let Some(target) = lines.get(idx + 1) {
                                        let _ = st.media_cmd_tx.try_send(MediaCommand::SeekMs(target.time_ms));
                                        handled = true;
                                    }
                                }
                            }
                        }
                    }
                }

                // 2. Focus Timer
                if !handled && st.sm.current == NotchState::FocusTimer {
                    let fz = focus_hit_zones(x, top_y, unit);
                    if fz.pause_btn.contains(cx, cy) {
                        st.pomodoro.toggle_run();
                        handled = true;
                    } else if fz.reset_btn.contains(cx, cy) {
                        st.pomodoro.reset();
                        handled = true;
                    }
                }

                // 3. Water & In-Notch Water Settings
                if !handled && st.sm.current == NotchState::Water {
                    let wz = water_hit_zones(x, top_y, unit);
                    if wz.reminder_switch.contains(cx, cy) {
                        st.config.water.reminder_enabled = !st.config.water.reminder_enabled;
                        let _ = config::save(&st.config);
                        handled = true;
                    } else if wz.drink_btn.contains(cx, cy) {
                        let bottle = st.config.water.bottle_ml;
                        if let Some(ref db) = st.db {
                            if let Ok(sum) = db.log_water(bottle) {
                                st.water = sum;
                            }
                        } else {
                            st.water.today_ml += bottle;
                        }
                        handled = true;
                    } else if wz.settings_btn.contains(cx, cy) {
                        st.sm.current = NotchState::InNotchSettings;
                        handled = true;
                    }
                } else if !handled && st.sm.current == NotchState::InNotchSettings {
                    let ws = water_settings_hit_zones(x, top_y, unit);
                    if ws.back_btn.contains(cx, cy) {
                        st.sm.current = NotchState::Water;
                        handled = true;
                    } else if ws.goal_minus.contains(cx, cy) {
                        st.config.water.daily_goal_ml = st.config.water.daily_goal_ml.saturating_sub(250).max(500);
                        let _ = config::save(&st.config);
                        handled = true;
                    } else if ws.goal_plus.contains(cx, cy) {
                        st.config.water.daily_goal_ml = (st.config.water.daily_goal_ml + 250).min(5000);
                        let _ = config::save(&st.config);
                        handled = true;
                    } else if ws.bottle_minus.contains(cx, cy) {
                        st.config.water.bottle_ml = st.config.water.bottle_ml.saturating_sub(50).max(100);
                        let _ = config::save(&st.config);
                        handled = true;
                    } else if ws.bottle_plus.contains(cx, cy) {
                        st.config.water.bottle_ml = (st.config.water.bottle_ml + 50).min(1000);
                        let _ = config::save(&st.config);
                        handled = true;
                    } else if ws.int_45.contains(cx, cy) {
                        st.config.water.reminder_interval_min = 45;
                        let _ = config::save(&st.config);
                        handled = true;
                    } else if ws.int_60.contains(cx, cy) {
                        st.config.water.reminder_interval_min = 60;
                        let _ = config::save(&st.config);
                        handled = true;
                    } else if ws.int_90.contains(cx, cy) {
                        st.config.water.reminder_interval_min = 90;
                        let _ = config::save(&st.config);
                        handled = true;
                    } else if ws.open_full_btn.contains(cx, cy) {
                        st.sm.current = NotchState::Water;
                        if let Some(app_inst) = win_weak.upgrade().and_then(|w| w.application()) {
                            let existing = st.settings_win.as_ref().and_then(|w| w.upgrade());
                            if let Some(sw) = existing {
                                sw.present();
                            } else {
                                let sw = build_settings_window(
                                    &app_inst,
                                    st.config.clone(),
                                    st.db.clone(),
                                    event_tx_click.clone(),
                                    Arc::clone(&ai_cfg_click),
                                    rt_handle_click.clone(),
                                    "focus",
                                );
                                st.settings_win = Some(sw.downgrade());
                                sw.present();
                            }
                        }
                        handled = true;
                    }
                }

                // 4. Shelf | Clipboard
                if !handled && matches!(st.sm.current, NotchState::Shelf | NotchState::Clipboard) {
                    let cz = frame.clipboard_hit_zones(top_y);
                    if cz.shelf_tab.contains(cx, cy) {
                        st.sm.toggle_shelf_clipboard(false);
                        handled = true;
                    } else if cz.clip_tab.contains(cx, cy) {
                        st.sm.toggle_shelf_clipboard(true);
                        handled = true;
                    } else if st.sm.current == NotchState::Shelf {
                        let sz = shelf_hit_zones(x, top_y, unit, st.shelf.len());
                        if sz.clear_btn.contains(cx, cy) {
                            if let Some(ref db) = st.db {
                                let _ = db.clear_shelf();
                            }
                            st.shelf.clear();
                            handled = true;
                        } else {
                            for (idx, rect) in sz.item_rects.iter().enumerate() {
                                if rect.contains(cx, cy) {
                                    if let Some(item) = st.shelf.get(idx) {
                                        let _ = std::process::Command::new("xdg-open")
                                            .arg(&item.uri)
                                            .spawn();
                                    }
                                    handled = true;
                                    break;
                                }
                            }
                        }
                    } else if st.sm.current == NotchState::Clipboard {
                        for (idx, rect) in cz.cards.iter().enumerate() {
                            if rect.contains(cx, cy) {
                                if let Some(entry) = st.clipboard.get(idx) {
                                    let _ = st.clip_copy_tx.try_send(entry.text.clone());
                                    st.copied_flash_idx = Some(idx);
                                    let rt_flash = Rc::clone(&rt);
                                    let area_flash = area_weak.clone();
                                    glib::timeout_add_local_once(Duration::from_millis(900), move || {
                                        rt_flash.borrow_mut().copied_flash_idx = None;
                                        if let Some(a) = area_flash.upgrade() {
                                            a.queue_draw();
                                        }
                                    });
                                }
                                handled = true;
                                break;
                            }
                        }
                    }
                }

                // 5. To-do
                if !handled && st.sm.current == NotchState::Todo {
                    let tz = todo_hit_zones(x, top_y, unit);
                    if tz.add_btn.contains(cx, cy) {
                        st.todo_entry_open = true;
                        st.sm.interactive_lock = true;
                        if let Some(entry) = entry_weak.upgrade() {
                            entry.set_visible(true);
                            entry.grab_focus();
                        }
                        handled = true;
                    } else if tz.edit_btn.contains(cx, cy) {
                        st.todo_edit_mode = !st.todo_edit_mode;
                        handled = true;
                    } else {
                        for (slot, rect) in tz.rows.iter().enumerate() {
                            if rect.contains(cx, cy) {
                                if let Some(task_id) = st.tasks.get(st.todo_scroll + slot).map(|t| t.id) {
                                    if let Some(ref db) = st.db {
                                        let res = if st.todo_edit_mode {
                                            db.delete_task(task_id)
                                        } else {
                                            db.toggle_task(task_id)
                                        };
                                        if let Ok(list) = res {
                                            st.tasks = list;
                                            let max_s = st.tasks.len().saturating_sub(3);
                                            st.todo_scroll = st.todo_scroll.min(max_s);
                                        }
                                    }
                                }
                                handled = true;
                                break;
                            }
                        }
                    }
                }

                // 6. Ask UMI AI Prompt (15)
                if !handled && st.sm.current == NotchState::AiPrompt {
                    let az = ai_prompt_hit_zones(x, top_y, unit);
                    if az.send_btn.contains(cx, cy) {
                        if let (Some(w), Some(ae)) = (win_weak.upgrade(), ai_weak.upgrade()) {
                            let prompt = ae.text().to_string();
                            if !prompt.trim().is_empty() {
                                ae.set_text("");
                                let req = AiRequest::Ask {
                                    prompt,
                                    translate_mode: st.ai_translate_mode,
                                };
                                st.submit_ai_request(req, &w, &ae);
                            }
                        }
                        handled = true;
                    } else if az.input_box.contains(cx, cy) {
                        if let Some(ae) = ai_weak.upgrade() {
                            ae.grab_focus();
                        }
                        handled = true;
                    } else if az.chip_summarize.contains(cx, cy) {
                        if let Some(ae) = ai_weak.upgrade() {
                            ae.set_text("Summarize: ");
                            ae.set_position(-1);
                            ae.grab_focus();
                        }
                        handled = true;
                    } else if az.chip_translate.contains(cx, cy) {
                        st.ai_translate_mode = !st.ai_translate_mode;
                        if let Some(ae) = ai_weak.upgrade() {
                            ae.grab_focus();
                        }
                        handled = true;
                    } else if az.chip_ocr.contains(cx, cy) {
                        if let (Some(w), Some(ae)) = (win_weak.upgrade(), ai_weak.upgrade()) {
                            st.submit_ai_request(AiRequest::SmartOcr, &w, &ae);
                        }
                        handled = true;
                    } else if az.chip_math.contains(cx, cy) {
                        if let (Some(w), Some(ae)) = (win_weak.upgrade(), ai_weak.upgrade()) {
                            st.ai_translate_mode = false;
                            st.submit_ai_request(
                                AiRequest::Ask {
                                    prompt: "100usd to idr=".into(),
                                    translate_mode: false,
                                },
                                &w,
                                &ae,
                            );
                        }
                        handled = true;
                    }
                }

                // 7. Ask UMI AI Answer (16)
                if !handled && st.sm.current == NotchState::AiAnswer {
                    let az = ai_answer_hit_zones(x, top_y, unit);
                    if az.mode_ask.contains(cx, cy) || az.mode_translate.contains(cx, cy) {
                        let new_tr = az.mode_translate.contains(cx, cy);
                        if st.ai_translate_mode != new_tr {
                            st.ai_translate_mode = new_tr;
                            if let Some(AiRequest::Ask { prompt, .. }) = st.ai_last_request.clone() {
                                if let (Some(w), Some(ae)) = (win_weak.upgrade(), ai_weak.upgrade()) {
                                    st.submit_ai_request(
                                        AiRequest::Ask {
                                            prompt,
                                            translate_mode: new_tr,
                                        },
                                        &w,
                                        &ae,
                                    );
                                }
                            }
                        }
                        handled = true;
                    } else if az.copy_btn.contains(cx, cy) {
                        if let Some(ref resp) = st.ai_response {
                            let _ = st.clip_copy_tx.try_send(resp.answer.clone());
                            st.ai_copied_flash = true;
                            let rt_f = Rc::clone(&rt);
                            let area_f = area_weak.clone();
                            glib::timeout_add_local_once(Duration::from_millis(900), move || {
                                rt_f.borrow_mut().ai_copied_flash = false;
                                if let Some(a) = area_f.upgrade() {
                                    a.queue_draw();
                                }
                            });
                        }
                        handled = true;
                    } else if az.retry_btn.contains(cx, cy) {
                        if let Some(last) = st.ai_last_request.clone() {
                            if let (Some(w), Some(ae)) = (win_weak.upgrade(), ai_weak.upgrade()) {
                                st.submit_ai_request(last, &w, &ae);
                            }
                        }
                        handled = true;
                    } else if az.followup_btn.contains(cx, cy) {
                        st.sm.current = NotchState::AiPrompt;
                        handled = true;
                    }
                }

                // 8. AI Limits (17) refresh pill
                if !handled && st.sm.current == NotchState::AiLimits {
                    let lz = ai_limits_hit_zones(x, top_y, unit);
                    if lz.refresh_pill.contains(cx, cy) {
                        let _ = st.quota_refresh_tx.try_send(());
                        handled = true;
                    }
                }

                // 9. Bottom pager dots (hoverable & clickable jump-to-page) + Left/Right arrows
                if !handled && !matches!(st.sm.current, NotchState::Lyrics | NotchState::InNotchSettings) {
                    let dot_rects =
                        frame.dot_hit_rects(top_y, st.sm.pages.len(), st.sm.active_page_idx);
                    if let Some(idx) = dot_rects.iter().position(|r| r.contains(cx, cy)) {
                        if let Some(&target_page) = st.sm.pages.get(idx) {
                            st.sm.interactive_lock = false;
                            if st.todo_entry_open {
                                st.todo_entry_open = false;
                                if let Some(e) = entry_weak.upgrade() {
                                    e.set_visible(false);
                                }
                            }
                            st.sm.jump_to_page(target_page);
                            handled = true;
                        }
                    }

                    if !handled {
                        let is_ai =
                            matches!(st.sm.current, NotchState::AiPrompt | NotchState::AiAnswer);
                        let x_off = if is_ai { 10.0 * unit } else { 0.0 };
                        let (lx, ly) = frame.left_arrow_center();
                        let (rx, ry) = frame.right_arrow_center();
                        let hit_r = if is_ai {
                            frame.arrow_radius() * 0.88 + 6.0
                        } else {
                            frame.arrow_radius() + 6.0
                        };
                        if (cx - (lx - x_off)).hypot(cy - (top_y + ly)) <= hit_r {
                            st.sm.interactive_lock = false;
                            if st.todo_entry_open {
                                st.todo_entry_open = false;
                                if let Some(e) = entry_weak.upgrade() {
                                    e.set_visible(false);
                                }
                            }
                            st.sm.prev_page();
                        } else if (cx - (rx + x_off)).hypot(cy - (top_y + ry)) <= hit_r {
                            st.sm.interactive_lock = false;
                            if st.todo_entry_open {
                                st.todo_entry_open = false;
                                if let Some(e) = entry_weak.upgrade() {
                                    e.set_visible(false);
                                }
                            }
                            st.sm.next_page();
                        }
                    }
                }
            }
            if let (Some(w), Some(ae)) = (win_weak.upgrade(), ai_weak.upgrade()) {
                st.sync_targets(&w, &ae);
            }
        });
    }
    drawing.add_controller(click);

    // Scroll wheel
    let scroll = EventControllerScroll::new(
        EventControllerScrollFlags::BOTH_AXES | EventControllerScrollFlags::DISCRETE,
    );
    {
        let rt = Rc::clone(&rt);
        let win_weak = window.downgrade();
        let ai_weak = ai_entry.downgrade();
        scroll.connect_scroll(move |_ctrl, dx, dy| {
            let mut st = rt.borrow_mut();
            if !st.sm.current.is_expanded_or_wide() {
                return glib::Propagation::Proceed;
            }
            if st.sm.current == NotchState::Todo && st.tasks.len() > 3 && dy.abs() > dx.abs() {
                let max_s = st.tasks.len() - 3;
                if dy > 0.0 && st.todo_scroll < max_s {
                    st.todo_scroll += 1;
                    if let (Some(w), Some(ae)) = (win_weak.upgrade(), ai_weak.upgrade()) {
                        st.sync_targets(&w, &ae);
                    }
                    return glib::Propagation::Stop;
                } else if dy < 0.0 && st.todo_scroll > 0 {
                    st.todo_scroll -= 1;
                    if let (Some(w), Some(ae)) = (win_weak.upgrade(), ai_weak.upgrade()) {
                        st.sync_targets(&w, &ae);
                    }
                    return glib::Propagation::Stop;
                }
            }
            let delta = if dx.abs() > dy.abs() { dx } else { dy };
            if delta > 0.0 {
                st.sm.interactive_lock = false;
                st.sm.next_page();
            } else if delta < 0.0 {
                st.sm.interactive_lock = false;
                st.sm.prev_page();
            }
            if let (Some(w), Some(ae)) = (win_weak.upgrade(), ai_weak.upgrade()) {
                st.sync_targets(&w, &ae);
            }
            glib::Propagation::Stop
        });
    }
    overlay.add_controller(scroll);

    // Window keyboard controller in CAPTURE phase so GtkEntry/GtkText never swallows Left/Right/Tab
    let keys = EventControllerKey::new();
    keys.set_propagation_phase(gtk4::PropagationPhase::Capture);
    {
        let rt = Rc::clone(&rt);
        let win_weak = window.downgrade();
        let entry_weak = todo_entry.downgrade();
        let ai_weak = ai_entry.downgrade();
        let event_tx_key = event_tx.clone();
        keys.connect_key_pressed(move |_ctrl, key, _code, mods| {
            if key == gdk::Key::comma && mods.contains(gdk::ModifierType::CONTROL_MASK) {
                let _ = event_tx_key.try_send(AppEvent::OpenSettings);
                return glib::Propagation::Stop;
            }
            let mut st = rt.borrow_mut();
            let ai_has_text = ai_weak
                .upgrade()
                .map(|e| !e.text().is_empty())
                .unwrap_or(false);
            let typing = st.todo_entry_open
                || (st.sm.current == NotchState::AiPrompt
                    && ai_has_text
                    && !mods.contains(gdk::ModifierType::ALT_MASK));
            match key {
                gdk::Key::Tab if st.sm.current == NotchState::AiPrompt => {
                    st.ai_translate_mode = !st.ai_translate_mode;
                    if let (Some(w), Some(ae)) = (win_weak.upgrade(), ai_weak.upgrade()) {
                        st.sync_targets(&w, &ae);
                    }
                    glib::Propagation::Stop
                }
                gdk::Key::Escape => {
                    if st.todo_entry_open {
                        st.todo_entry_open = false;
                        st.sm.interactive_lock = false;
                        if let Some(e) = entry_weak.upgrade() {
                            e.set_text("");
                            e.set_visible(false);
                        }
                    } else if st.sm.current == NotchState::Lyrics {
                        st.sm.toggle_lyrics();
                    } else if st.sm.current == NotchState::InNotchSettings {
                        st.sm.current = NotchState::Water;
                    } else if st.sm.current == NotchState::AiAnswer {
                        st.sm.current = NotchState::AiPrompt;
                    } else {
                        if let Some(ae) = ai_weak.upgrade() {
                            ae.set_text("");
                        }
                        st.sm.collapse(true);
                    }
                    if let (Some(w), Some(ae)) = (win_weak.upgrade(), ai_weak.upgrade()) {
                        st.sync_targets(&w, &ae);
                    }
                    glib::Propagation::Stop
                }
                gdk::Key::Left if !typing => {
                    st.sm.interactive_lock = false;
                    st.sm.prev_page();
                    if let (Some(w), Some(ae)) = (win_weak.upgrade(), ai_weak.upgrade()) {
                        st.sync_targets(&w, &ae);
                    }
                    glib::Propagation::Stop
                }
                gdk::Key::Right if !typing => {
                    st.sm.interactive_lock = false;
                    st.sm.next_page();
                    if let (Some(w), Some(ae)) = (win_weak.upgrade(), ai_weak.upgrade()) {
                        st.sync_targets(&w, &ae);
                    }
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        });
    }
    window.add_controller(keys);

    // Event loop for IPC, Tray, KDE Watchers, Ask UMI AI, Smart OCR, AI Limits & Settings
    {
        let rt = Rc::clone(&rt);
        let win_weak = window.downgrade();
        let area_weak = drawing.downgrade();
        let todo_weak = todo_entry.downgrade();
        let ai_weak = ai_entry.downgrade();
        let app_weak = app.downgrade();
        let event_tx_ev = event_tx.clone();
        let ai_cfg_ev = Arc::clone(&ai_cfg);
        let rt_handle_ev = rt_handle.clone();
        glib::spawn_future_local(async move {
            while let Ok(ev) = event_rx.recv().await {
                let mut st = rt.borrow_mut();
                let mut triggered_peek = false;

                match ev {
                    AppEvent::ToggleNotch => {
                        st.cancel_timers();
                        st.sm.toggle_expand();
                    }
                    AppEvent::OpenAskAi => {
                        st.cancel_timers();
                        st.sm.open_ask_ai();
                    }
                    AppEvent::TriggerOcr => {
                        st.cancel_timers();
                        if let (Some(w), Some(ae)) = (win_weak.upgrade(), ai_weak.upgrade()) {
                            st.submit_ai_request(AiRequest::SmartOcr, &w, &ae);
                            continue;
                        }
                    }
                    AppEvent::OpenSettings => {
                        if let Some(app_inst) = app_weak.upgrade() {
                            let existing = st.settings_win.as_ref().and_then(|w| w.upgrade());
                            if let Some(sw) = existing {
                                sw.present();
                            } else {
                                let sw = build_settings_window(
                                    &app_inst,
                                    st.config.clone(),
                                    st.db.clone(),
                                    event_tx_ev.clone(),
                                    Arc::clone(&ai_cfg_ev),
                                    rt_handle_ev.clone(),
                                    "general",
                                );
                                st.settings_win = Some(sw.downgrade());
                                sw.present();
                            }
                        }
                    }
                    AppEvent::JumpToPage(page) => {
                        st.cancel_timers();
                        st.sm.jump_to_page(page);
                    }
                    AppEvent::ToggleStartAtLogin => {
                        st.config.general.start_at_login = !st.config.general.start_at_login;
                        st.tray_flags
                            .start_at_login
                            .store(st.config.general.start_at_login, Ordering::Relaxed);
                        let _ = crate::sys::desktop::sync_autostart(st.config.general.start_at_login);
                        let _ = config::save(&st.config);
                    }
                    AppEvent::ToggleMascotEyes => {
                        st.config.general.idle_mascot_eyes = !st.config.general.idle_mascot_eyes;
                        let enabled = st.config.general.idle_mascot_eyes;
                        st.tray_flags.mascot_eyes.store(enabled, Ordering::Relaxed);
                        st.sm.set_mascot_enabled(enabled);
                        let _ = config::save(&st.config);
                    }
                    AppEvent::ToggleMinimalMode => {
                        st.config.general.minimal_notch_mode =
                            !st.config.general.minimal_notch_mode;
                        st.tray_flags
                            .minimal_mode
                            .store(st.config.general.minimal_notch_mode, Ordering::Relaxed);
                        let _ = config::save(&st.config);
                    }
                    AppEvent::BatteryUpdated {
                        pct,
                        charging,
                        just_plugged_in,
                    } => {
                        st.battery_pct = pct;
                        st.battery_charging = charging;
                        if just_plugged_in {
                            st.sm.enqueue_peek(NotchState::Charging { battery_pct: pct });
                            triggered_peek = st.sm.current.is_peek();
                        }
                    }
                    AppEvent::BluetoothConnected {
                        device,
                        battery_pct,
                    } => {
                        st.sm.enqueue_peek(NotchState::Headphones {
                            device,
                            battery_pct,
                        });
                        triggered_peek = st.sm.current.is_peek();
                    }
                    AppEvent::NotificationReceived { app, sender, body } => {
                        st.sm.enqueue_peek(NotchState::Notification { app, sender, body });
                        triggered_peek = st.sm.current.is_peek();
                    }
                    AppEvent::MediaUpdated(media) => {
                        let playing = media.playing;
                        st.media = media;
                        st.refresh_album_art();
                        st.sm.set_media_playing(playing);
                    }
                    AppEvent::LyricsUpdated { track_key, data } => {
                        if st.lyrics_track_key != track_key {
                            st.lyrics_track_key = track_key;
                            st.lyrics_scroll_idx = 0.0;
                            st.lyrics_scroll_vel = 0.0;
                        }
                        st.lyrics = data;
                    }
                    AppEvent::ClipboardUpdated(entries) => {
                        st.clipboard = entries;
                    }
                    AppEvent::PrayerLocationDetected {
                        city,
                        latitude,
                        longitude,
                    } => {
                        st.config.prayer.city = city;
                        st.config.prayer.latitude = latitude;
                        st.config.prayer.longitude = longitude;
                        st.prayer = compute_today_schedule(&st.config.prayer);
                        let _ = config::save(&st.config);
                    }
                    AppEvent::AiResponseReady(resp) => {
                        st.ai_loading = false;
                        if resp.auto_copy {
                            let _ = st.clip_copy_tx.try_send(resp.answer.clone());
                        }
                        st.ai_response = Some(resp);
                        st.sm.jump_to_page(NotchPageId::AskAi);
                        st.sm.current = NotchState::AiAnswer;
                    }
                    AppEvent::QuotasUpdated(snaps) => {
                        st.quotas = snaps;
                    }
                    AppEvent::QuotaLowAlert {
                        provider,
                        remaining_pct,
                        resets_in,
                    } => {
                        st.sm.enqueue_peek(NotchState::QuotaLow {
                            provider,
                            remaining_pct,
                            resets_in,
                        });
                        triggered_peek = st.sm.current.is_peek();
                    }
                    AppEvent::ConfigChanged(new_cfg) => {
                        let scale_changed =
                            (new_cfg.appearance.scale - st.config.appearance.scale).abs() > 0.005;
                        let style_changed = new_cfg.appearance.style != st.config.appearance.style;
                        let layer_changed =
                            new_cfg.general.hide_in_fullscreen != st.config.general.hide_in_fullscreen;
                        let monitor_changed =
                            new_cfg.appearance.display_monitor != st.config.appearance.display_monitor;
                        let quota_changed =
                            new_cfg.ai.quota_providers != st.config.ai.quota_providers;
                        st.config = new_cfg;
                        if quota_changed {
                            let _ = st.quota_refresh_tx.try_send(());
                        }
                        if style_changed {
                            st.last_input_rect = None;
                        }
                        if let Some(w) = win_weak.upgrade() {
                            if layer_changed {
                                w.set_layer(if st.config.general.hide_in_fullscreen {
                                    Layer::Top
                                } else {
                                    Layer::Overlay
                                });
                            }
                            if monitor_changed {
                                apply_target_monitor(&w, st.config.appearance.display_monitor.as_deref());
                            }
                        }
                        st.tray_flags
                            .start_at_login
                            .store(st.config.general.start_at_login, Ordering::Relaxed);
                        st.tray_flags
                            .mascot_eyes
                            .store(st.config.general.idle_mascot_eyes, Ordering::Relaxed);
                        st.tray_flags
                            .minimal_mode
                            .store(st.config.general.minimal_notch_mode, Ordering::Relaxed);
                        let mascot_on = st.config.general.idle_mascot_eyes;
                        st.sm.set_mascot_enabled(mascot_on);
                        if !st.config.pages.active.is_empty() {
                            st.sm.pages = st.config.pages.active.clone();
                            if st.sm.active_page_idx >= st.sm.pages.len() {
                                st.sm.active_page_idx = 0;
                            }
                        }
                        if !st.pomodoro.running {
                            st.pomodoro = PomodoroEngine::new(st.config.focus.clone());
                        }
                        st.prayer = compute_today_schedule(&st.config.prayer);

                        if scale_changed {
                            let sc = st.config.appearance.scale;
                            let u = sc / 1.3;
                            let (cw, ch) = canvas_dimensions(sc);
                            if let Some(a) = area_weak.upgrade() {
                                a.set_content_width(cw);
                                a.set_content_height(ch);
                            }
                            if let Some(w) = win_weak.upgrade() {
                                w.set_default_size(cw, ch);
                            }
                            if let Some(te) = todo_weak.upgrade() {
                                te.set_width_request((230.0 * sc) as i32);
                                te.set_margin_top((86.0 * sc) as i32);
                            }
                            if let Some(ae) = ai_weak.upgrade() {
                                ae.set_width_request((392.0 * u) as i32);
                                ae.set_margin_top((48.0 * u) as i32);
                                ae.set_margin_end((16.0 * u) as i32);
                            }
                            st.last_input_rect = None;
                        }
                    }
                    AppEvent::Quit => {
                        if let Some(a) = app_weak.upgrade() {
                            a.quit();
                        }
                        break;
                    }
                }

                if triggered_peek {
                    schedule_peek_dismiss(&rt, &mut st, &win_weak, &ai_weak);
                }

                if let (Some(w), Some(ae)) = (win_weak.upgrade(), ai_weak.upgrade()) {
                    st.sync_targets(&w, &ae);
                }
            }
        });
    }

    window.set_child(Some(&overlay));
    window
}

fn schedule_peek_dismiss(
    rt: &Rc<RefCell<NotchRuntime>>,
    st: &mut NotchRuntime,
    win_weak: &glib::WeakRef<ApplicationWindow>,
    ai_weak: &glib::WeakRef<Entry>,
) {
    if let Some(id) = st.pending_peek_dismiss.take() {
        id.remove();
    }
    let rt_cb = Rc::clone(rt);
    let win_cb = win_weak.clone();
    let ai_cb = ai_weak.clone();
    let id = glib::timeout_add_local_once(Duration::from_millis(4200), move || {
        let mut st = rt_cb.borrow_mut();
        st.pending_peek_dismiss = None;
        if st.sm.current.is_peek() {
            st.sm.dismiss_peek();
            if st.sm.current.is_peek() {
                schedule_peek_dismiss(&rt_cb, &mut st, &win_cb, &ai_cb);
            }
            if let (Some(w), Some(ae)) = (win_cb.upgrade(), ai_cb.upgrade()) {
                st.sync_targets(&w, &ae);
            }
        }
    });
    st.pending_peek_dismiss = Some(id);
}

fn load_image_surface(path: &Path) -> Option<ImageSurface> {
    let texture = gdk::Texture::from_filename(path).ok()?;
    let w = texture.width();
    let h = texture.height();
    if w <= 0 || h <= 0 {
        return None;
    }
    let mut surface = ImageSurface::create(Format::ARgb32, w, h).ok()?;
    let stride = surface.stride() as usize;
    {
        let mut buf = surface.data().ok()?;
        texture.download(&mut buf, stride);
    }
    // Downscale large album thumbnails (e.g. 1280x720 YouTube art) to <= 128x128 (~64 KB instead of ~4 MB)
    const MAX_DIM: i32 = 128;
    if w <= MAX_DIM && h <= MAX_DIM {
        return Some(surface);
    }
    let scale = (MAX_DIM as f64 / w.max(h) as f64).min(1.0);
    let dw = ((w as f64 * scale).round() as i32).max(1);
    let dh = ((h as f64 * scale).round() as i32).max(1);
    let small = ImageSurface::create(Format::ARgb32, dw, dh).ok()?;
    let cr = gtk4::cairo::Context::new(&small).ok()?;
    cr.scale(dw as f64 / w as f64, dh as f64 / h as f64);
    let _ = cr.set_source_surface(&surface, 0.0, 0.0);
    let _ = cr.paint();
    drop(cr);
    Some(small)
}

fn update_input_region(
    window: &ApplicationWindow,
    st: &mut NotchRuntime,
    canvas_w: f64,
    use_target: bool,
) {
    if canvas_w < 100.0 {
        return;
    }
    let Some(surface) = window.surface() else {
        return;
    };
    let scale = st.config.appearance.scale;
    let w = (if use_target {
        st.spring.width.target
    } else {
        st.spring.width.value
    }) * scale;
    let h = (if use_target {
        st.spring.height.target
    } else {
        st.spring.height.value
    }) * scale;
    let ear = (if use_target {
        st.spring.ear_radius.target
    } else {
        st.spring.ear_radius.value
    }) * scale;
    let top_pad = if st.config.appearance.style == crate::config::NotchStyle::Floating {
        6.0 * scale
    } else {
        0.0
    };

    let rx = ((canvas_w - w) * 0.5 - ear).floor().max(0.0) as i32;
    let ry = 0;
    let rw = (w + 2.0 * ear).ceil().max(1.0) as i32;
    let rh = (h + top_pad + 4.0).ceil().max(1.0) as i32;
    let next = (rx, ry, rw, rh);

    if st.last_input_rect != Some(next) {
        let region = Region::create_rectangle(&RectangleInt::new(rx, ry, rw, rh));
        surface.set_input_region(&region);
        st.last_input_rect = Some(next);
    }
}

fn apply_target_monitor(window: &ApplicationWindow, connector: Option<&str>) {
    let Some(display) = gdk::Display::default() else {
        return;
    };
    let monitors = display.monitors();
    if let Some(target) = connector {
        for i in 0..monitors.n_items() {
            if let Some(mon) = monitors
                .item(i)
                .and_then(|o| o.downcast::<gdk::Monitor>().ok())
            {
                if mon.connector().as_deref() == Some(target) {
                    window.set_monitor(&mon);
                    return;
                }
            }
        }
    }
    if let Some(primary) = monitors
        .item(0)
        .and_then(|o| o.downcast::<gdk::Monitor>().ok())
    {
        window.set_monitor(&primary);
    }
}

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_string(include_str!("../../../assets/styles/notch.css"));
    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
