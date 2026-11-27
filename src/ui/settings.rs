use crate::config::{
    self, AiConfig, FaithTradition, MascotEyeTint, NotchPageId, NotchStyle, SpringSpeed, UmiConfig,
    WidgetKind,
};
use crate::domain::prayer_calc::compute_today_schedule;
use crate::state::AppEvent;
use crate::storage::{secrets, LocalDb};
use crate::sys::desktop;
use crate::ui::notch::renderer::accent_aura_stops;
use directories::BaseDirs;
use gtk4::cairo::{self, Context, LinearGradient};
use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Box as GtkBox, Button, CssProvider, DrawingArea,
    DropDown, Entry, HeaderBar, Label, Orientation, PasswordEntry, Scale, ScrolledWindow,
    Separator, Stack, StackTransitionType, StringList, Switch,
};
use std::cell::RefCell;
use std::f64::consts::PI;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

const CITIES: &[(&str, f64, f64)] = &[
    ("Surabaya", -7.2914, 112.7960),
    ("Jakarta", -6.2088, 106.8456),
    ("Bandung", -6.9175, 107.6191),
    ("Yogyakarta", -7.7956, 110.3695),
    ("Semarang", -6.9667, 110.4167),
    ("Malang", -7.9797, 112.6304),
    ("Medan", 3.5952, 98.6722),
    ("Makassar", -5.1477, 119.4327),
    ("Denpasar", -8.6705, 115.2126),
    ("Singapore", 1.3521, 103.8198),
    ("Kuala Lumpur", 3.1390, 101.6869),
    ("Makkah", 21.4225, 39.8262),
];

const METHODS: &[(&str, &str)] = &[
    ("kemenag", "Kemenag RI (20° / 18°)"),
    ("mwl", "Muslim World League (18° / 17°)"),
    ("isna", "ISNA (15° / 15°)"),
    ("umm_al_qura", "Umm al-Qura (18.5° / 90m)"),
];

const ALL_PAGES: &[(NotchPageId, &str, &str, &str)] = &[
    (
        NotchPageId::Music,
        "Music & Synced Lyrics",
        "Spotify, YouTube, YouTube Music + LRCLIB lyrics",
        "#38BDF8",
    ),
    (
        NotchPageId::Focus,
        "Focus Timer (Pomodoro)",
        "Round timeline with live glowing progress head",
        "#818CF8",
    ),
    (
        NotchPageId::Prayer,
        "Prayer Times",
        "Sun arc & local Adzan calculation",
        "#FBBF24",
    ),
    (
        NotchPageId::Water,
        "Water Reminder",
        "Daily goal tracking & hourly intake bars",
        "#2DD4BF",
    ),
    (
        NotchPageId::Shelf,
        "File Shelf & Clipboard History",
        "Drag-and-drop file paths + recent copied snippets",
        "#A855F7",
    ),
    (
        NotchPageId::Todo,
        "To-do List & Calendar",
        "Daily tasks & due reminders inside notch",
        "#34D399",
    ),
    (
        NotchPageId::Widgets,
        "Widgets (2 Slots)",
        "Calendar, Battery, Weather, World Clock",
        "#60A5FA",
    ),
    (
        NotchPageId::AskAi,
        "Ask UMI AI",
        "Multi-provider BYOK chat, Translate, Quick Math & OCR",
        "#C084FC",
    ),
    (
        NotchPageId::AiLimits,
        "AI Limits",
        "Antigravity, Codex, Claude & custom ScriptProvider quotas",
        "#F59E0B",
    ),
];

#[derive(Clone)]
struct SettingsCtx {
    cfg: Rc<RefCell<UmiConfig>>,
    event_tx: async_channel::Sender<AppEvent>,
    ai_cfg: Arc<RwLock<AiConfig>>,
    rt_handle: tokio::runtime::Handle,
    db: Option<LocalDb>,
    preview_area: Rc<RefCell<Option<DrawingArea>>>,
    style_btns: Rc<RefCell<Vec<(NotchStyle, Button)>>>,
}

impl SettingsCtx {
    fn commit(&self) {
        let snap = self.cfg.borrow().clone();
        if let Ok(mut guard) = self.ai_cfg.write() {
            *guard = snap.ai.clone();
        }
        let _ = self.event_tx.try_send(AppEvent::ConfigChanged(snap.clone()));
        if let Some(ref area) = *self.preview_area.borrow() {
            area.queue_draw();
        }
        // Persist TOML off the GTK UI thread so style/color clicks react with 0ms latency
        self.rt_handle.spawn_blocking(move || {
            if let Err(err) = config::save(&snap) {
                tracing::warn!("Failed saving config.toml: {err}");
            }
        });
    }

    fn sync_style_buttons(&self, style: NotchStyle) {
        for (s_id, btn) in self.style_btns.borrow().iter() {
            if *s_id == style {
                btn.add_css_class("active");
            } else {
                btn.remove_css_class("active");
            }
        }
    }
}

pub fn build_settings_window(
    app: &Application,
    initial_cfg: UmiConfig,
    db: Option<LocalDb>,
    event_tx: async_channel::Sender<AppEvent>,
    ai_cfg: Arc<RwLock<AiConfig>>,
    rt_handle: tokio::runtime::Handle,
    initial_tab: &str,
) -> ApplicationWindow {
    load_settings_css();

    let ctx = SettingsCtx {
        cfg: Rc::new(RefCell::new(initial_cfg)),
        event_tx,
        ai_cfg,
        rt_handle,
        db,
        preview_area: Rc::new(RefCell::new(None)),
        style_btns: Rc::new(RefCell::new(Vec::new())),
    };

    let window = ApplicationWindow::builder()
        .application(app)
        .title("UMI Settings")
        .default_width(860)
        .default_height(604)
        .resizable(true)
        .build();
    window.add_css_class("umi-settings-window");

    // Titlebar (44px) — uses native KDE/GTK org.umi.Notch icon + clean "UMI Settings" label
    let header = HeaderBar::new();
    header.add_css_class("umi-settings-header");
    header.set_show_title_buttons(true);
    let title_lbl = Label::new(Some("UMI Settings"));
    title_lbl.add_css_class("umi-win-title");
    title_lbl.set_margin_start(4);
    header.set_title_widget(Some(&GtkBox::new(Orientation::Horizontal, 0)));
    header.pack_start(&title_lbl);
    window.set_titlebar(Some(&header));

    // Main horizontal split: Sidebar (224px) | Content Stack (635px)
    let root = GtkBox::new(Orientation::Horizontal, 0);

    let sidebar = GtkBox::new(Orientation::Vertical, 6);
    sidebar.add_css_class("umi-sidebar");
    sidebar.set_width_request(224);
    sidebar.set_hexpand(false);

    // Sidebar brand header (clean logo + UMI wordmark, no version pill)
    let brand_row = GtkBox::new(Orientation::Horizontal, 9);
    brand_row.set_margin_bottom(12);
    brand_row.set_margin_start(4);
    brand_row.set_valign(Align::Center);
    brand_row.append(&make_logo_widget(28));
    let brand_lbl = Label::new(Some("UMI"));
    brand_lbl.add_css_class("umi-brand-name");
    brand_row.append(&brand_lbl);
    sidebar.append(&brand_row);

    let stack = Stack::new();
    stack.set_hexpand(true);
    stack.set_vexpand(true);
    stack.set_transition_type(StackTransitionType::Crossfade);
    stack.set_transition_duration(130);

    let nav_specs: &[(&str, &str, &str, &str)] = &[
        ("general", "General", "⚙", "umi-badge-general"),
        ("appearance", "Appearance", "✦", "umi-badge-appearance"),
        ("pages", "Notch Pages", "▤", "umi-badge-pages"),
        ("focus", "Focus & Water", "◷", "umi-badge-focus"),
        ("prayer", "Prayer & Devotion", "☾", "umi-badge-prayer"),
        ("ai", "AI & Shortcuts", "✧", "umi-badge-ai"),
        ("about", "About & Data", "◈", "umi-badge-about"),
        ("widgets", "Widgets & Calendar", "▦", "umi-badge-widgets"),
    ];

    let nav_buttons: Rc<RefCell<Vec<(String, Button)>>> = Rc::new(RefCell::new(Vec::new()));
    let stack_for_select = stack.clone();
    let nav_btns_for_select = Rc::clone(&nav_buttons);
    let select_tab: Rc<dyn Fn(&str)> = Rc::new(move |id: &str| {
        stack_for_select.set_visible_child_name(id);
        for (bid, btn) in nav_btns_for_select.borrow().iter() {
            if bid == id {
                btn.add_css_class("active");
            } else {
                btn.remove_css_class("active");
            }
        }
    });

    for (id, label, glyph, badge_cls) in nav_specs {
        let btn = Button::new();
        btn.add_css_class("umi-nav-btn");
        let row = GtkBox::new(Orientation::Horizontal, 10);
        row.set_valign(Align::Center);
        let badge = Label::new(Some(glyph));
        badge.add_css_class("umi-badge");
        badge.add_css_class(badge_cls);
        badge.set_halign(Align::Center);
        badge.set_valign(Align::Center);
        let txt = Label::new(Some(label));
        txt.set_xalign(0.0);
        row.append(&badge);
        row.append(&txt);
        btn.set_child(Some(&row));

        let id_owned = (*id).to_string();
        let select_cb = Rc::clone(&select_tab);
        btn.connect_clicked(move |_| {
            select_cb(&id_owned);
        });
        sidebar.append(&btn);
        nav_buttons.borrow_mut().push(((*id).to_string(), btn));
    }

    // Privacy footer pinned to bottom of sidebar
    let spacer = GtkBox::new(Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    sidebar.append(&spacer);
    let privacy = Label::new(Some("🔒  Stored locally · No tracking"));
    privacy.add_css_class("umi-privacy-footer");
    privacy.set_xalign(0.0);
    privacy.set_margin_start(6);
    privacy.set_margin_bottom(2);
    sidebar.append(&privacy);

    // Build all 8 screens (matching unified FigCraft Settings Feature)
    stack.add_named(&wrap_scroll(build_screen_general(&ctx, Rc::clone(&select_tab))), Some("general"));
    stack.add_named(&wrap_scroll(build_screen_appearance(&ctx)), Some("appearance"));
    stack.add_named(&wrap_scroll(build_screen_pages(&ctx)), Some("pages"));
    stack.add_named(&wrap_scroll(build_screen_focus_water(&ctx)), Some("focus"));
    stack.add_named(&wrap_scroll(build_screen_prayer(&ctx)), Some("prayer"));
    stack.add_named(&wrap_scroll(build_screen_ai(&ctx)), Some("ai"));
    stack.add_named(&wrap_scroll(build_screen_about(&ctx)), Some("about"));
    stack.add_named(&wrap_scroll(build_screen_widgets_calendar(&ctx)), Some("widgets"));

    select_tab(initial_tab);

    root.append(&sidebar);
    root.append(&stack);
    window.set_child(Some(&root));
    window
}

fn wrap_scroll(content: GtkBox) -> ScrolledWindow {
    content.add_css_class("umi-content-wrap");
    let sc = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .hexpand(true)
        .vexpand(true)
        .child(&content)
        .build();
    sc
}

fn page_header(title: &str, subtitle: &str) -> GtkBox {
    let bx = GtkBox::new(Orientation::Vertical, 3);
    let t = Label::new(Some(title));
    t.add_css_class("umi-page-title");
    t.set_xalign(0.0);
    let s = Label::new(Some(subtitle));
    s.add_css_class("umi-page-sub");
    s.set_xalign(0.0);
    s.set_wrap(true);
    bx.append(&t);
    bx.append(&s);
    bx
}

fn section_label(text: &str) -> Label {
    let l = Label::new(Some(text));
    l.add_css_class("umi-sec-label");
    l.set_xalign(0.0);
    l
}

fn make_toggle_row<F>(title: &str, sub: &str, initial: bool, on_change: F) -> GtkBox
where
    F: Fn(bool) + 'static,
{
    let row = GtkBox::new(Orientation::Horizontal, 12);
    row.set_valign(Align::Center);
    let left = GtkBox::new(Orientation::Vertical, 2);
    left.set_hexpand(true);
    let t = Label::new(Some(title));
    t.add_css_class("umi-row-title");
    t.set_xalign(0.0);
    let s = Label::new(Some(sub));
    s.add_css_class("umi-row-sub");
    s.set_xalign(0.0);
    s.set_wrap(true);
    left.append(&t);
    left.append(&s);

    let sw = Switch::new();
    sw.set_active(initial);
    sw.set_valign(Align::Center);
    sw.connect_state_set(move |_s, state| {
        on_change(state);
        glib::Propagation::Proceed
    });

    row.append(&left);
    row.append(&sw);
    row
}

fn make_sep() -> Separator {
    let sep = Separator::new(Orientation::Horizontal);
    sep.add_css_class("umi-sep");
    sep
}

/// Screen 1 · General & Notch Behavior
fn build_screen_general(ctx: &SettingsCtx, select_tab: Rc<dyn Fn(&str)>) -> GtkBox {
    let page = GtkBox::new(Orientation::Vertical, 12);
    page.append(&page_header(
        "General & Notch Behavior",
        "Control startup behavior, idle mascot appearance, and notch geometry.",
    ));

    // Live Notch Preview Card
    let preview_card = GtkBox::new(Orientation::Vertical, 8);
    preview_card.add_css_class("umi-preview-box");

    let preview_top = GtkBox::new(Orientation::Horizontal, 8);
    let tag_left = Label::new(Some("Live Preview · Resting"));
    tag_left.add_css_class("umi-row-title");
    tag_left.set_hexpand(true);
    tag_left.set_xalign(0.0);
    let tag_right = Label::new(Some("Hover top edge on desktop to expand"));
    tag_right.add_css_class("umi-row-sub");
    preview_top.append(&tag_left);
    preview_top.append(&tag_right);
    preview_card.append(&preview_top);

    let preview_mode = Rc::new(RefCell::new(0u8)); // 0=Resting, 1=Closed, 2=Minimal
    let init_st = ctx.cfg.borrow().appearance.style;
    let init_fil = ctx.cfg.borrow().appearance.smooth_concave_fillets;
    let preview_aura = Rc::new(RefCell::new(accent_aura_stops(
        &ctx.cfg.borrow().appearance.accent_aura,
    )));
    let preview_eye = Rc::new(RefCell::new(ctx.cfg.borrow().appearance.eye_tint.rgb()));
    // (float_mix, ear_mix, glass_mix, nw, nh)
    let preview_geom = Rc::new(RefCell::new((
        if init_st == NotchStyle::Floating { 1.0 } else { 0.0 },
        if init_fil && init_st != NotchStyle::Floating { 1.0 } else { 0.0 },
        if init_st == NotchStyle::Glass { 1.0 } else { 0.0 },
        186.0_f64,
        38.0_f64,
    )));
    let preview_da = DrawingArea::new();
    preview_da.set_content_height(62);
    preview_da.set_hexpand(true);
    {
        let mode_rc = Rc::clone(&preview_mode);
        let aura_rc = Rc::clone(&preview_aura);
        let eye_rc = Rc::clone(&preview_eye);
        let geom_rc = Rc::clone(&preview_geom);
        preview_da.set_draw_func(move |_da, cr, w, h| {
            let (fm, em, gm, nw, nh) = *geom_rc.borrow();
            draw_live_notch_preview(
                cr,
                w as f64,
                h as f64,
                *mode_rc.borrow(),
                *aura_rc.borrow(),
                *eye_rc.borrow(),
                fm,
                em,
                gm,
                nw,
                nh,
            );
        });
    }
    {
        let cfg_rc = Rc::clone(&ctx.cfg);
        let mode_rc = Rc::clone(&preview_mode);
        let aura_rc = Rc::clone(&preview_aura);
        let eye_rc = Rc::clone(&preview_eye);
        let geom_rc = Rc::clone(&preview_geom);
        let last_us = Rc::new(RefCell::new(None::<i64>));
        preview_da.add_tick_callback(move |da, clock| {
            let now_us = clock.frame_time();
            let dt = {
                let mut prev = last_us.borrow_mut();
                let d = match *prev {
                    Some(p) if now_us > p => ((now_us - p) as f64) / 1_000_000.0,
                    _ => 1.0 / 60.0,
                }
                .clamp(0.001, 0.05);
                *prev = Some(now_us);
                d
            };
            let ck = 1.0 - (-12.0 * dt).exp();
            let sk = 1.0 - (-16.0 * dt).exp();
            let (t_aura, t_eye, t_style, t_fil) = {
                let cfg = cfg_rc.borrow();
                (
                    accent_aura_stops(&cfg.appearance.accent_aura),
                    cfg.appearance.eye_tint.rgb(),
                    cfg.appearance.style,
                    cfg.appearance.smooth_concave_fillets,
                )
            };
            let mode = *mode_rc.borrow();
            let (t_nw, t_nh) = if mode == 2 { (124.0, 8.0) } else { (186.0, 38.0) };
            let t_fm = if t_style == NotchStyle::Floating { 1.0 } else { 0.0 };
            let t_em = if t_fil && t_style != NotchStyle::Floating { 1.0 } else { 0.0 };
            let t_gm = if t_style == NotchStyle::Glass { 1.0 } else { 0.0 };

            let mut moving = false;
            {
                let mut a = aura_rc.borrow_mut();
                for i in 0..3 {
                    let (tr, tg, tb) = t_aura[i];
                    let (cr, cg, cb) = a[i];
                    if (tr - cr).abs() > 0.0015 || (tg - cg).abs() > 0.0015 || (tb - cb).abs() > 0.0015 {
                        a[i] = (cr + (tr - cr) * ck, cg + (tg - cg) * ck, cb + (tb - cb) * ck);
                        moving = true;
                    } else {
                        a[i] = (tr, tg, tb);
                    }
                }
            }
            {
                let mut e = eye_rc.borrow_mut();
                let (tr, tg, tb) = t_eye;
                let (cr, cg, cb) = *e;
                if (tr - cr).abs() > 0.0015 || (tg - cg).abs() > 0.0015 || (tb - cb).abs() > 0.0015 {
                    *e = (cr + (tr - cr) * ck, cg + (tg - cg) * ck, cb + (tb - cb) * ck);
                    moving = true;
                } else {
                    *e = (tr, tg, tb);
                }
            }
            {
                let g = &mut *geom_rc.borrow_mut();
                let mut step_g = |cur: &mut f64, tgt: f64, eps: f64| {
                    if (*cur - tgt).abs() > eps {
                        *cur += (tgt - *cur) * sk;
                        moving = true;
                    } else {
                        *cur = tgt;
                    }
                };
                step_g(&mut g.0, t_fm, 0.002);
                step_g(&mut g.1, t_em, 0.002);
                step_g(&mut g.2, t_gm, 0.002);
                step_g(&mut g.3, t_nw, 0.2);
                step_g(&mut g.4, t_nh, 0.2);
            }
            if moving {
                da.queue_draw();
            }
            glib::ControlFlow::Continue
        });
    }
    *ctx.preview_area.borrow_mut() = Some(preview_da.clone());
    preview_card.append(&preview_da);

    let mode_row = GtkBox::new(Orientation::Horizontal, 6);
    mode_row.set_halign(Align::Center);
    let mode_btns: Rc<RefCell<Vec<Button>>> = Rc::new(RefCell::new(Vec::new()));
    for (idx, lbl) in [(0u8, "Resting"), (1u8, "Closed"), (2u8, "Minimal")] {
        let b = Button::with_label(lbl);
        b.add_css_class("umi-seg-btn");
        if idx == 0 {
            b.add_css_class("active");
        }
        let m_rc = Rc::clone(&preview_mode);
        let da_c = preview_da.clone();
        let tl = tag_left.clone();
        let mbs = Rc::clone(&mode_btns);
        b.connect_clicked(move |_| {
            *m_rc.borrow_mut() = idx;
            tl.set_text(&format!("Live Preview · {lbl}"));
            for (bi, btn) in mbs.borrow().iter().enumerate() {
                if bi as u8 == idx {
                    btn.add_css_class("active");
                } else {
                    btn.remove_css_class("active");
                }
            }
            da_c.queue_draw();
        });
        mode_row.append(&b);
        mode_btns.borrow_mut().push(b);
    }
    preview_card.append(&mode_row);
    page.append(&preview_card);

    // Core General Toggles Card
    let toggles = GtkBox::new(Orientation::Vertical, 10);
    toggles.add_css_class("umi-card");
    let cfg_init = ctx.cfg.borrow().clone();

    let c1 = ctx.clone();
    toggles.append(&make_toggle_row(
        "Start at Login",
        "Launch UMI silently in the system tray on sign-in (~/.config/autostart/umi.desktop)",
        cfg_init.general.start_at_login,
        move |v| {
            c1.cfg.borrow_mut().general.start_at_login = v;
            let _ = desktop::sync_autostart(v);
            c1.commit();
        },
    ));
    toggles.append(&make_sep());

    let c2 = ctx.clone();
    toggles.append(&make_toggle_row(
        "Idle Mascot Eyes",
        "Clock fades aside and UMI mascot appears when quiet",
        cfg_init.general.idle_mascot_eyes,
        move |v| {
            c2.cfg.borrow_mut().general.idle_mascot_eyes = v;
            c2.commit();
        },
    ));
    toggles.append(&make_sep());

    let c3 = ctx.clone();
    toggles.append(&make_toggle_row(
        "Minimal Notch Mode",
        "Collapse closed notch to a thin bezel line for thick bezels",
        cfg_init.general.minimal_notch_mode,
        move |v| {
            c3.cfg.borrow_mut().general.minimal_notch_mode = v;
            c3.commit();
        },
    ));
    toggles.append(&make_sep());

    let c4 = ctx.clone();
    toggles.append(&make_toggle_row(
        "Show Date & Clock in Header",
        "Display live date and time across expanded notch pages",
        cfg_init.general.show_header_clock,
        move |v| {
            c4.cfg.borrow_mut().general.show_header_clock = v;
            c4.commit();
        },
    ));
    toggles.append(&make_sep());

    let c5 = ctx.clone();
    toggles.append(&make_toggle_row(
        "Hide Behind Fullscreen Apps",
        "Use Wayland Top layer so fullscreen videos & games cover the notch automatically",
        cfg_init.general.hide_in_fullscreen,
        move |v| {
            c5.cfg.borrow_mut().general.hide_in_fullscreen = v;
            c5.commit();
        },
    ));
    toggles.append(&make_sep());

    // Multi-monitor display selector
    {
        let mut connectors: Vec<Option<String>> = vec![None];
        let mut labels: Vec<String> = vec!["Primary Display (Auto)".to_string()];
        if let Some(display) = gdk::Display::default() {
            let mons = display.monitors();
            for i in 0..mons.n_items() {
                if let Some(mon) = mons.item(i).and_then(|o| o.downcast::<gdk::Monitor>().ok()) {
                    if let Some(conn) = mon.connector() {
                        let model = mon
                            .model()
                            .map(|m| format!(" · {m}"))
                            .unwrap_or_default();
                        labels.push(format!("{conn}{model}"));
                        connectors.push(Some(conn.to_string()));
                    }
                }
            }
        }
        let row = GtkBox::new(Orientation::Horizontal, 8);
        let lbl = Label::new(Some("Target Display"));
        lbl.add_css_class("umi-row-title");
        lbl.set_hexpand(true);
        lbl.set_xalign(0.0);
        let str_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
        let dd = DropDown::new(Some(StringList::new(&str_refs)), None::<gtk4::Expression>);
        let init_idx = connectors
            .iter()
            .position(|c| *c == cfg_init.appearance.display_monitor)
            .unwrap_or(0) as u32;
        dd.set_selected(init_idx);
        let c = ctx.clone();
        dd.connect_selected_notify(move |d| {
            if let Some(conn) = connectors.get(d.selected() as usize) {
                c.cfg.borrow_mut().appearance.display_monitor = conn.clone();
                c.commit();
            }
        });
        row.append(&lbl);
        row.append(&dd);
        toggles.append(&row);
    }
    page.append(&toggles);

    // Quick summary strip: Style | Active Pages | Ask UMI Shortcut
    let summary_card = GtkBox::new(Orientation::Vertical, 10);
    summary_card.add_css_class("umi-card");

    // Notch Style quick switcher
    let style_row = GtkBox::new(Orientation::Horizontal, 8);
    let sl = Label::new(Some("Notch Style"));
    sl.add_css_class("umi-row-title");
    sl.set_hexpand(true);
    sl.set_xalign(0.0);
    style_row.append(&sl);
    for (st, name) in [
        (NotchStyle::Attached, "Attached"),
        (NotchStyle::Floating, "Floating"),
        (NotchStyle::Glass, "Glass"),
    ] {
        let b = Button::with_label(name);
        b.add_css_class("umi-seg-btn");
        if cfg_init.appearance.style == st {
            b.add_css_class("active");
        }
        let c = ctx.clone();
        b.connect_clicked(move |_| {
            c.cfg.borrow_mut().appearance.style = st;
            c.sync_style_buttons(st);
            c.commit();
        });
        style_row.append(&b);
        ctx.style_btns.borrow_mut().push((st, b));
    }
    summary_card.append(&style_row);
    summary_card.append(&make_sep());

    // Active Pages chips + Edit...
    let pages_row = GtkBox::new(Orientation::Horizontal, 6);
    let pl = Label::new(Some("Active Pages"));
    pl.add_css_class("umi-row-title");
    pl.set_hexpand(true);
    pl.set_xalign(0.0);
    pages_row.append(&pl);
    for p in cfg_init.pages.active.iter().take(5) {
        let chip = Label::new(Some(p.title()));
        chip.add_css_class("umi-chip");
        pages_row.append(&chip);
    }
    let edit_btn = Button::with_label("Edit...");
    edit_btn.add_css_class("umi-action-btn");
    let sel_pages = Rc::clone(&select_tab);
    edit_btn.connect_clicked(move |_| sel_pages("pages"));
    pages_row.append(&edit_btn);
    summary_card.append(&pages_row);
    summary_card.append(&make_sep());

    // Ask UMI Shortcut keycaps
    let sc_row = GtkBox::new(Orientation::Horizontal, 6);
    let scl = Label::new(Some("Ask UMI Shortcut"));
    scl.add_css_class("umi-row-title");
    scl.set_hexpand(true);
    scl.set_xalign(0.0);
    sc_row.append(&scl);
    for k in ["Ctrl", "Alt", "Space"] {
        let cap = Label::new(Some(k));
        cap.add_css_class("umi-keycap");
        sc_row.append(&cap);
    }
    summary_card.append(&sc_row);

    page.append(&summary_card);
    page
}

/// Screen 2 · Appearance & Themes
fn build_screen_appearance(ctx: &SettingsCtx) -> GtkBox {
    let page = GtkBox::new(Orientation::Vertical, 12);
    page.append(&page_header(
        "Appearance & Themes",
        "Customize notch shape, accent aura, glass translucency & mascot style.",
    ));

    let cfg_init = ctx.cfg.borrow().clone();

    // Notch Style & Accent Aura card
    let card1 = GtkBox::new(Orientation::Vertical, 12);
    card1.add_css_class("umi-card");

    let st_row = GtkBox::new(Orientation::Horizontal, 8);
    let st_lbl = Label::new(Some("Notch Style"));
    st_lbl.add_css_class("umi-row-title");
    st_lbl.set_hexpand(true);
    st_lbl.set_xalign(0.0);
    st_row.append(&st_lbl);
    for (st, name) in [
        (NotchStyle::Attached, "Attached"),
        (NotchStyle::Floating, "Floating"),
        (NotchStyle::Glass, "Glass"),
    ] {
        let b = Button::with_label(name);
        b.add_css_class("umi-seg-btn");
        if cfg_init.appearance.style == st {
            b.add_css_class("active");
        }
        let c = ctx.clone();
        b.connect_clicked(move |_| {
            c.cfg.borrow_mut().appearance.style = st;
            c.sync_style_buttons(st);
            c.commit();
        });
        st_row.append(&b);
        ctx.style_btns.borrow_mut().push((st, b));
    }
    card1.append(&st_row);
    card1.append(&make_sep());

    // 6 Accent Aura swatches
    let aura_row = GtkBox::new(Orientation::Horizontal, 8);
    let aura_lbl = Label::new(Some("Accent Aura"));
    aura_lbl.add_css_class("umi-row-title");
    aura_lbl.set_hexpand(true);
    aura_lbl.set_xalign(0.0);
    aura_row.append(&aura_lbl);
    let swatch_btns: Rc<RefCell<Vec<(String, Button)>>> = Rc::new(RefCell::new(Vec::new()));
    for (id, css) in [
        ("sky", "umi-swatch-sky"),
        ("rose", "umi-swatch-rose"),
        ("mint", "umi-swatch-mint"),
        ("amber", "umi-swatch-amber"),
        ("ocean", "umi-swatch-ocean"),
        ("mono", "umi-swatch-mono"),
    ] {
        let b = Button::new();
        b.add_css_class("umi-swatch-btn");
        b.add_css_class(css);
        if cfg_init.appearance.accent_aura == id {
            b.add_css_class("active");
        }
        let c = ctx.clone();
        let sws = Rc::clone(&swatch_btns);
        let id_s = id.to_string();
        b.connect_clicked(move |_| {
            c.cfg.borrow_mut().appearance.accent_aura = id_s.clone();
            for (sid, btn) in sws.borrow().iter() {
                if sid == &id_s {
                    btn.add_css_class("active");
                } else {
                    btn.remove_css_class("active");
                }
            }
            c.commit();
        });
        aura_row.append(&b);
        swatch_btns.borrow_mut().push((id.to_string(), b));
    }
    card1.append(&aura_row);
    card1.append(&make_sep());

    // Mascot Eye Tint
    let tint_row = GtkBox::new(Orientation::Horizontal, 6);
    let tint_lbl = Label::new(Some("Mascot Eye Tint"));
    tint_lbl.add_css_class("umi-row-title");
    tint_lbl.set_hexpand(true);
    tint_lbl.set_xalign(0.0);
    tint_row.append(&tint_lbl);
    let tint_btns: Rc<RefCell<Vec<(MascotEyeTint, Button)>>> = Rc::new(RefCell::new(Vec::new()));
    for (tint, name) in [
        (MascotEyeTint::SkyGlint, "Sky Glint"),
        (MascotEyeTint::Mint, "Mint"),
        (MascotEyeTint::Rose, "Rose"),
        (MascotEyeTint::White, "White"),
    ] {
        let b = Button::with_label(name);
        b.add_css_class("umi-seg-btn");
        if cfg_init.appearance.eye_tint == tint {
            b.add_css_class("active");
        }
        let c = ctx.clone();
        let tbs = Rc::clone(&tint_btns);
        b.connect_clicked(move |_| {
            c.cfg.borrow_mut().appearance.eye_tint = tint;
            for (tid, btn) in tbs.borrow().iter() {
                if *tid == tint {
                    btn.add_css_class("active");
                } else {
                    btn.remove_css_class("active");
                }
            }
            c.commit();
        });
        tint_row.append(&b);
        tint_btns.borrow_mut().push((tint, b));
    }
    card1.append(&tint_row);
    page.append(&card1);

    // Geometry & Motion Card
    let card2 = GtkBox::new(Orientation::Vertical, 12);
    card2.add_css_class("umi-card");

    // Glass Blur Slider
    let blur_row = GtkBox::new(Orientation::Horizontal, 10);
    let bl = Label::new(Some("Glass Background Blur"));
    bl.add_css_class("umi-row-title");
    bl.set_hexpand(true);
    bl.set_xalign(0.0);
    let blur_val = Label::new(Some(&format!("{} px", cfg_init.appearance.glass_blur_px)));
    blur_val.add_css_class("umi-chip");
    let blur_scale = Scale::with_range(Orientation::Horizontal, 0.0, 40.0, 2.0);
    blur_scale.set_width_request(180);
    blur_scale.set_draw_value(false);
    blur_scale.set_value(cfg_init.appearance.glass_blur_px as f64);
    {
        let c = ctx.clone();
        let bv = blur_val.clone();
        blur_scale.connect_value_changed(move |sc| {
            let px = sc.value().round() as u32;
            bv.set_text(&format!("{px} px"));
            c.cfg.borrow_mut().appearance.glass_blur_px = px;
            c.commit();
        });
    }
    blur_row.append(&bl);
    blur_row.append(&blur_scale);
    blur_row.append(&blur_val);
    card2.append(&blur_row);
    card2.append(&make_sep());

    // Notch Display Scale Slider (80% - 150%)
    let scale_row = GtkBox::new(Orientation::Horizontal, 10);
    let scl = Label::new(Some("Notch Display Scale"));
    scl.add_css_class("umi-row-title");
    scl.set_hexpand(true);
    scl.set_xalign(0.0);
    let scale_pct = (cfg_init.appearance.scale * 100.0).round() as i32;
    let scale_val = Label::new(Some(&format!("{scale_pct}%")));
    scale_val.add_css_class("umi-chip");
    let scale_slider = Scale::with_range(Orientation::Horizontal, 80.0, 150.0, 5.0);
    scale_slider.set_width_request(180);
    scale_slider.set_draw_value(false);
    scale_slider.set_value(scale_pct as f64);
    {
        let c = ctx.clone();
        let sv = scale_val.clone();
        scale_slider.connect_value_changed(move |sc| {
            let pct = sc.value().round() as i32;
            sv.set_text(&format!("{pct}%"));
            c.cfg.borrow_mut().appearance.scale = (pct as f64) / 100.0;
            c.commit();
        });
    }
    scale_row.append(&scl);
    scale_row.append(&scale_slider);
    scale_row.append(&scale_val);
    card2.append(&scale_row);
    card2.append(&make_sep());

    // Smooth Concave Fillets
    let c_fil = ctx.clone();
    card2.append(&make_toggle_row(
        "Smooth Concave Fillets",
        "Blend upper-left and upper-right notch ears smoothly into the top screen edge",
        cfg_init.appearance.smooth_concave_fillets,
        move |v| {
            c_fil.cfg.borrow_mut().appearance.smooth_concave_fillets = v;
            c_fil.commit();
        },
    ));
    card2.append(&make_sep());

    // Spring Animation Speed
    let sp_row = GtkBox::new(Orientation::Horizontal, 6);
    let sp_lbl = Label::new(Some("Spring Animation Speed"));
    sp_lbl.add_css_class("umi-row-title");
    sp_lbl.set_hexpand(true);
    sp_lbl.set_xalign(0.0);
    sp_row.append(&sp_lbl);
    let sp_btns: Rc<RefCell<Vec<(SpringSpeed, Button)>>> = Rc::new(RefCell::new(Vec::new()));
    for (sp, name) in [
        (SpringSpeed::Snappy, "Snappy"),
        (SpringSpeed::Balanced, "Balanced"),
        (SpringSpeed::Calm, "Calm"),
    ] {
        let b = Button::with_label(name);
        b.add_css_class("umi-seg-btn");
        if cfg_init.appearance.spring_speed == sp {
            b.add_css_class("active");
        }
        let c = ctx.clone();
        let sps = Rc::clone(&sp_btns);
        b.connect_clicked(move |_| {
            c.cfg.borrow_mut().appearance.spring_speed = sp;
            for (sid, btn) in sps.borrow().iter() {
                if *sid == sp {
                    btn.add_css_class("active");
                } else {
                    btn.remove_css_class("active");
                }
            }
            c.commit();
        });
        sp_row.append(&b);
        sp_btns.borrow_mut().push((sp, b));
    }
    card2.append(&sp_row);
    card2.append(&make_sep());

    let c_dim = ctx.clone();
    card2.append(&make_toggle_row(
        "Dim Screen Behind Expanded Notch",
        "Subtle contrast emphasis when expanded panels are active",
        cfg_init.appearance.dim_behind_notch,
        move |v| {
            c_dim.cfg.borrow_mut().appearance.dim_behind_notch = v;
            c_dim.commit();
        },
    ));

    page.append(&card2);
    page
}

/// Screen 3 · Notch Pages & Order
fn build_screen_pages(ctx: &SettingsCtx) -> GtkBox {
    let page = GtkBox::new(Orientation::Vertical, 12);
    page.append(&page_header(
        "Notch Pages & Order",
        "Enable, disable, and reorder the interactive pages inside your notch.",
    ));

    let list_box = GtkBox::new(Orientation::Vertical, 8);
    let list_ref = Rc::new(list_box.clone());

    let rebuild: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));
    let rebuild_inner: Rc<dyn Fn()> = {
        let ctx = ctx.clone();
        let list_ref = Rc::clone(&list_ref);
        let rebuild_weak = Rc::clone(&rebuild);
        Rc::new(move || {
            while let Some(child) = list_ref.first_child() {
                list_ref.remove(&child);
            }
            let active = ctx.cfg.borrow().pages.active.clone();

            // Ordered display: active pages first (in user order), then disabled pages
            let mut ordered: Vec<NotchPageId> = active.clone();
            for (pid, _, _, _) in ALL_PAGES {
                if !ordered.contains(pid) {
                    ordered.push(*pid);
                }
            }

            for (pos, pid) in ordered.iter().copied().enumerate() {
                let Some((_, title, sub, hex)) = ALL_PAGES.iter().find(|(p, _, _, _)| *p == pid)
                else {
                    continue;
                };
                let is_on = active.contains(&pid);
                let active_pos = active.iter().position(|p| *p == pid);

                let card = GtkBox::new(Orientation::Horizontal, 10);
                card.add_css_class("umi-card");
                card.set_valign(Align::Center);

                // Colored module dot
                let dot = DrawingArea::new();
                dot.set_content_width(10);
                dot.set_content_height(10);
                dot.set_valign(Align::Center);
                let hex_s = (*hex).to_string();
                dot.set_draw_func(move |_, cr, w, h| {
                    let (r, g, b) = parse_hex(&hex_s);
                    cr.arc(w as f64 * 0.5, h as f64 * 0.5, 4.5, 0.0, 2.0 * PI);
                    cr.set_source_rgb(r, g, b);
                    let _ = cr.fill();
                });

                let txt_col = GtkBox::new(Orientation::Vertical, 2);
                txt_col.set_hexpand(true);
                let t = Label::new(Some(title));
                t.add_css_class("umi-row-title");
                t.set_xalign(0.0);
                let s = Label::new(Some(sub));
                s.add_css_class("umi-row-sub");
                s.set_xalign(0.0);
                txt_col.append(&t);
                txt_col.append(&s);

                // Reorder up / down buttons for active pages
                let up_btn = Button::with_label("↑");
                up_btn.add_css_class("umi-seg-btn");
                up_btn.set_sensitive(is_on && active_pos.unwrap_or(0) > 0);
                {
                    let c = ctx.clone();
                    let rb = Rc::clone(&rebuild_weak);
                    up_btn.connect_clicked(move |_| {
                        {
                            let mut cfg = c.cfg.borrow_mut();
                            if let Some(i) = cfg.pages.active.iter().position(|p| *p == pid) {
                                if i > 0 {
                                    cfg.pages.active.swap(i, i - 1);
                                }
                            }
                        }
                        c.commit();
                        if let Some(f) = rb.borrow().as_ref() {
                            f();
                        }
                    });
                }

                let dn_btn = Button::with_label("↓");
                dn_btn.add_css_class("umi-seg-btn");
                dn_btn.set_sensitive(
                    is_on && active_pos.map(|i| i + 1 < active.len()).unwrap_or(false),
                );
                {
                    let c = ctx.clone();
                    let rb = Rc::clone(&rebuild_weak);
                    dn_btn.connect_clicked(move |_| {
                        {
                            let mut cfg = c.cfg.borrow_mut();
                            if let Some(i) = cfg.pages.active.iter().position(|p| *p == pid) {
                                if i + 1 < cfg.pages.active.len() {
                                    cfg.pages.active.swap(i, i + 1);
                                }
                            }
                        }
                        c.commit();
                        if let Some(f) = rb.borrow().as_ref() {
                            f();
                        }
                    });
                }

                let sw = Switch::new();
                sw.set_active(is_on);
                sw.set_valign(Align::Center);
                {
                    let c = ctx.clone();
                    let rb = Rc::clone(&rebuild_weak);
                    sw.connect_state_set(move |_s, state| {
                        let changed = {
                            let mut cfg = c.cfg.borrow_mut();
                            if state {
                                if !cfg.pages.active.contains(&pid) {
                                    cfg.pages.active.push(pid);
                                }
                                true
                            } else if cfg.pages.active.len() > 1 {
                                cfg.pages.active.retain(|p| *p != pid);
                                true
                            } else {
                                false
                            }
                        };
                        if changed {
                            c.commit();
                            if let Some(f) = rb.borrow().as_ref() {
                                f();
                            }
                            glib::Propagation::Proceed
                        } else {
                            glib::Propagation::Stop
                        }
                    });
                }

                let idx_lbl = Label::new(Some(&format!("{}.", pos + 1)));
                idx_lbl.add_css_class("umi-row-sub");
                idx_lbl.set_width_request(18);

                card.append(&idx_lbl);
                card.append(&dot);
                card.append(&txt_col);
                card.append(&up_btn);
                card.append(&dn_btn);
                card.append(&sw);
                list_ref.append(&card);
            }
        })
    };
    *rebuild.borrow_mut() = Some(Rc::clone(&rebuild_inner));
    rebuild_inner();

    page.append(&list_box);

    let footer = GtkBox::new(Orientation::Horizontal, 8);
    let hint = Label::new(Some("Use ↑ / ↓ to reorder swipe & dot order inside the notch"));
    hint.add_css_class("umi-row-sub");
    hint.set_hexpand(true);
    hint.set_xalign(0.0);
    let reset_btn = Button::with_label("Reset Default");
    reset_btn.add_css_class("umi-action-btn");
    {
        let c = ctx.clone();
        let rb = Rc::clone(&rebuild);
        reset_btn.connect_clicked(move |_| {
            c.cfg.borrow_mut().pages = config::PagesConfig::default();
            c.commit();
            if let Some(f) = rb.borrow().as_ref() {
                f();
            }
        });
    }
    footer.append(&hint);
    footer.append(&reset_btn);
    page.append(&footer);

    page
}

/// Screen 4 · Focus & Water Reminder
fn build_screen_focus_water(ctx: &SettingsCtx) -> GtkBox {
    let page = GtkBox::new(Orientation::Vertical, 10);
    page.append(&page_header(
        "Focus & Water Reminder",
        "Configure Pomodoro round lengths, daily hydration goal, and bottle size.",
    ));

    let cfg_init = ctx.cfg.borrow().clone();

    // Pomodoro card
    page.append(&section_label("POMODORO FOCUS TIMER"));
    let pomo_card = GtkBox::new(Orientation::Vertical, 10);
    pomo_card.add_css_class("umi-card");

    // Focus Session Length stepper
    {
        let row = GtkBox::new(Orientation::Horizontal, 8);
        let lbl = Label::new(Some("Focus Session Length"));
        lbl.add_css_class("umi-row-title");
        lbl.set_hexpand(true);
        lbl.set_xalign(0.0);
        let val = Label::new(Some(&format!("{} min", cfg_init.focus.focus_min)));
        val.add_css_class("umi-chip");
        let minus = Button::with_label("-");
        minus.add_css_class("umi-seg-btn");
        let plus = Button::with_label("+");
        plus.add_css_class("umi-seg-btn");
        {
            let c = ctx.clone();
            let v = val.clone();
            minus.connect_clicked(move |_| {
                let next = {
                    let mut cfg = c.cfg.borrow_mut();
                    cfg.focus.focus_min = cfg.focus.focus_min.saturating_sub(5).max(5);
                    cfg.focus.focus_min
                };
                v.set_text(&format!("{next} min"));
                c.commit();
            });
        }
        {
            let c = ctx.clone();
            let v = val.clone();
            plus.connect_clicked(move |_| {
                let next = {
                    let mut cfg = c.cfg.borrow_mut();
                    cfg.focus.focus_min = (cfg.focus.focus_min + 5).min(90);
                    cfg.focus.focus_min
                };
                v.set_text(&format!("{next} min"));
                c.commit();
            });
        }
        row.append(&lbl);
        row.append(&minus);
        row.append(&val);
        row.append(&plus);
        pomo_card.append(&row);
    }
    pomo_card.append(&make_sep());

    // Short / Long Break stepper pills
    {
        let row = GtkBox::new(Orientation::Horizontal, 8);
        let lbl = Label::new(Some("Short / Long Break"));
        lbl.add_css_class("umi-row-title");
        lbl.set_hexpand(true);
        lbl.set_xalign(0.0);
        let sb_btn = Button::with_label(&format!("Short: {} min", cfg_init.focus.short_break_min));
        sb_btn.add_css_class("umi-seg-btn");
        {
            let c = ctx.clone();
            let b_ref = sb_btn.clone();
            sb_btn.connect_clicked(move |_| {
                let next = {
                    let mut cfg = c.cfg.borrow_mut();
                    cfg.focus.short_break_min = match cfg.focus.short_break_min {
                        3 => 5,
                        5 => 10,
                        _ => 3,
                    };
                    cfg.focus.short_break_min
                };
                b_ref.set_label(&format!("Short: {next} min"));
                c.commit();
            });
        }
        let lb_btn = Button::with_label(&format!("Long: {} min", cfg_init.focus.long_break_min));
        lb_btn.add_css_class("umi-seg-btn");
        {
            let c = ctx.clone();
            let b_ref = lb_btn.clone();
            lb_btn.connect_clicked(move |_| {
                let next = {
                    let mut cfg = c.cfg.borrow_mut();
                    cfg.focus.long_break_min = match cfg.focus.long_break_min {
                        10 => 15,
                        15 => 20,
                        20 => 30,
                        _ => 10,
                    };
                    cfg.focus.long_break_min
                };
                b_ref.set_label(&format!("Long: {next} min"));
                c.commit();
            });
        }
        row.append(&lbl);
        row.append(&sb_btn);
        row.append(&lb_btn);
        pomo_card.append(&row);
    }
    pomo_card.append(&make_sep());

    // Sessions per Round
    {
        let row = GtkBox::new(Orientation::Horizontal, 6);
        let lbl = Label::new(Some("Sessions per Round"));
        lbl.add_css_class("umi-row-title");
        lbl.set_hexpand(true);
        lbl.set_xalign(0.0);
        row.append(&lbl);
        let s_btns: Rc<RefCell<Vec<(u32, Button)>>> = Rc::new(RefCell::new(Vec::new()));
        for n in [2u32, 3, 4, 6] {
            let b = Button::with_label(&n.to_string());
            b.add_css_class("umi-seg-btn");
            if cfg_init.focus.sessions_per_round == n {
                b.add_css_class("active");
            }
            let c = ctx.clone();
            let sbs = Rc::clone(&s_btns);
            b.connect_clicked(move |_| {
                c.cfg.borrow_mut().focus.sessions_per_round = n;
                for (nid, btn) in sbs.borrow().iter() {
                    if *nid == n {
                        btn.add_css_class("active");
                    } else {
                        btn.remove_css_class("active");
                    }
                }
                c.commit();
            });
            row.append(&b);
            s_btns.borrow_mut().push((n, b));
        }
        pomo_card.append(&row);
    }
    pomo_card.append(&make_sep());

    let c_auto = ctx.clone();
    pomo_card.append(&make_toggle_row(
        "Auto-start Next Session",
        "Automatically advance from break back into Focus",
        cfg_init.focus.auto_start_next,
        move |v| {
            c_auto.cfg.borrow_mut().focus.auto_start_next = v;
            c_auto.commit();
        },
    ));
    page.append(&pomo_card);

    // Water Reminder Card
    page.append(&section_label("WATER REMINDER & HYDRATION"));
    let water_card = GtkBox::new(Orientation::Vertical, 10);
    water_card.add_css_class("umi-card");

    // Daily Hydration Goal stepper
    {
        let row = GtkBox::new(Orientation::Horizontal, 8);
        let lbl = Label::new(Some("Daily Hydration Goal"));
        lbl.add_css_class("umi-row-title");
        lbl.set_hexpand(true);
        lbl.set_xalign(0.0);
        let val = Label::new(Some(&format!(
            "{:.2} L",
            cfg_init.water.daily_goal_ml as f64 / 1000.0
        )));
        val.add_css_class("umi-chip");
        let minus = Button::with_label("-");
        minus.add_css_class("umi-seg-btn");
        let plus = Button::with_label("+");
        plus.add_css_class("umi-seg-btn");
        {
            let c = ctx.clone();
            let v = val.clone();
            minus.connect_clicked(move |_| {
                let ml = {
                    let mut cfg = c.cfg.borrow_mut();
                    cfg.water.daily_goal_ml = cfg.water.daily_goal_ml.saturating_sub(250).max(500);
                    cfg.water.daily_goal_ml
                };
                v.set_text(&format!("{:.2} L", ml as f64 / 1000.0));
                c.commit();
            });
        }
        {
            let c = ctx.clone();
            let v = val.clone();
            plus.connect_clicked(move |_| {
                let ml = {
                    let mut cfg = c.cfg.borrow_mut();
                    cfg.water.daily_goal_ml = (cfg.water.daily_goal_ml + 250).min(5000);
                    cfg.water.daily_goal_ml
                };
                v.set_text(&format!("{:.2} L", ml as f64 / 1000.0));
                c.commit();
            });
        }
        row.append(&lbl);
        row.append(&minus);
        row.append(&val);
        row.append(&plus);
        water_card.append(&row);
    }
    water_card.append(&make_sep());

    // Bottle / Cup Size
    {
        let row = GtkBox::new(Orientation::Horizontal, 6);
        let lbl = Label::new(Some("Bottle / Cup Size"));
        lbl.add_css_class("umi-row-title");
        lbl.set_hexpand(true);
        lbl.set_xalign(0.0);
        row.append(&lbl);
        let b_btns: Rc<RefCell<Vec<(u32, Button)>>> = Rc::new(RefCell::new(Vec::new()));
        for ml in [200u32, 250, 350, 500] {
            let b = Button::with_label(&format!("{ml} ml"));
            b.add_css_class("umi-seg-btn");
            if cfg_init.water.bottle_ml == ml {
                b.add_css_class("active");
            }
            let c = ctx.clone();
            let bbs = Rc::clone(&b_btns);
            b.connect_clicked(move |_| {
                c.cfg.borrow_mut().water.bottle_ml = ml;
                for (mid, btn) in bbs.borrow().iter() {
                    if *mid == ml {
                        btn.add_css_class("active");
                    } else {
                        btn.remove_css_class("active");
                    }
                }
                c.commit();
            });
            row.append(&b);
            b_btns.borrow_mut().push((ml, b));
        }
        water_card.append(&row);
    }
    water_card.append(&make_sep());

    // Reminder Interval
    {
        let row = GtkBox::new(Orientation::Horizontal, 6);
        let lbl = Label::new(Some("Reminder Interval"));
        lbl.add_css_class("umi-row-title");
        lbl.set_hexpand(true);
        lbl.set_xalign(0.0);
        row.append(&lbl);
        let i_btns: Rc<RefCell<Vec<(u32, Button)>>> = Rc::new(RefCell::new(Vec::new()));
        for m in [45u32, 60, 90] {
            let b = Button::with_label(&format!("{m} min"));
            b.add_css_class("umi-seg-btn");
            if cfg_init.water.reminder_interval_min == m {
                b.add_css_class("active");
            }
            let c = ctx.clone();
            let ibs = Rc::clone(&i_btns);
            b.connect_clicked(move |_| {
                c.cfg.borrow_mut().water.reminder_interval_min = m;
                for (mid, btn) in ibs.borrow().iter() {
                    if *mid == m {
                        btn.add_css_class("active");
                    } else {
                        btn.remove_css_class("active");
                    }
                }
                c.commit();
            });
            row.append(&b);
            i_btns.borrow_mut().push((m, b));
        }
        water_card.append(&row);
    }
    water_card.append(&make_sep());

    let c_gs = ctx.clone();
    water_card.append(&make_toggle_row(
        "Gentle Reminder Sound",
        "Play soft hydration chime when water nudge triggers",
        cfg_init.water.gentle_sound,
        move |v| {
            c_gs.cfg.borrow_mut().water.gentle_sound = v;
            c_gs.commit();
        },
    ));

    page.append(&water_card);
    page
}

/// Screen 5 · Prayer Times & Location
/// Screen 5 · Multi-Faith Prayer & Devotion (6 Traditions in Indonesia — FigCraft 05 & 05b)
fn build_screen_prayer(ctx: &SettingsCtx) -> GtkBox {
    let page = GtkBox::new(Orientation::Vertical, 12);
    page.append(&page_header(
        "Prayer & Devotion (Multi-Faith)",
        "Select faith tradition (6 faiths in Indonesia), offline schedule, location & media auto-pause.",
    ));

    let cfg_init = ctx.cfg.borrow().clone();

    // 6-Faith Selector Strip (Islam, Kristen, Katolik, Hindu, Buddha, Khonghucu)
    let faith_strip = GtkBox::new(Orientation::Horizontal, 6);
    faith_strip.set_homogeneous(true);
    let faith_btns: Rc<RefCell<Vec<(FaithTradition, Button)>>> = Rc::new(RefCell::new(Vec::new()));

    // Dynamic Schedule Banner
    let sched_hdr = section_label("TODAY'S LOCAL SCHEDULE");
    let sched_strip = GtkBox::new(Orientation::Horizontal, 8);
    sched_strip.set_homogeneous(true);
    let feast_banner = Label::new(None);
    feast_banner.add_css_class("umi-row-sub");
    feast_banner.set_xalign(0.0);

    // Conditional Islam vs Non-Islam containers
    let islam_box = GtkBox::new(Orientation::Vertical, 10);
    let other_box = GtkBox::new(Orientation::Vertical, 10);
    let custom_entry = Entry::new();
    custom_entry.add_css_class("umi-settings-entry");
    custom_entry.set_hexpand(true);

    let sync_custom_entry_text = {
        let ce = custom_entry.clone();
        let cfg_rc = Rc::clone(&ctx.cfg);
        Rc::new(move || {
            let p = cfg_rc.borrow().prayer.clone();
            let slots: Vec<String> = p
                .faith
                .default_slots()
                .iter()
                .enumerate()
                .map(|(i, (_, def))| {
                    p.custom_slots
                        .get(i)
                        .filter(|s| !s.trim().is_empty())
                        .cloned()
                        .unwrap_or_else(|| (*def).to_string())
                })
                .collect();
            ce.set_text(&slots.join(", "));
        })
    };

    let render_schedule = {
        let strip = sched_strip.clone();
        let hdr = sched_hdr.clone();
        let fb = feast_banner.clone();
        let ib = islam_box.clone();
        let ob = other_box.clone();
        let cfg_rc = Rc::clone(&ctx.cfg);
        Rc::new(move || {
            while let Some(c) = strip.first_child() {
                strip.remove(&c);
            }
            let p_cfg = cfg_rc.borrow().prayer.clone();
            let sched = compute_today_schedule(&p_cfg);
            let is_islam = p_cfg.faith == FaithTradition::Islam;
            ib.set_visible(is_islam);
            ob.set_visible(!is_islam);

            if is_islam {
                hdr.set_text(&format!(
                    "TODAY'S LOCAL SCHEDULE · {} (SUN ARC)",
                    p_cfg.city.to_uppercase()
                ));
            } else {
                hdr.set_text(&format!(
                    "TODAY'S SCHEDULE · {} ({})",
                    p_cfg.faith.notch_header().0.to_uppercase(),
                    p_cfg.faith.label().to_uppercase()
                ));
            }

            if let Some(ref hint) = sched.feast_hint {
                fb.set_text(hint);
                fb.set_visible(true);
            } else {
                fb.set_visible(false);
            }

            for (i, entry) in sched.entries.iter().enumerate() {
                let slot = GtkBox::new(Orientation::Vertical, 3);
                slot.add_css_class("umi-prayer-slot");
                if i == sched.next_idx {
                    slot.add_css_class("next");
                }
                let n = Label::new(Some(entry.name));
                n.add_css_class("umi-prayer-name");
                let t = Label::new(Some(&entry.hh_mm()));
                t.add_css_class("umi-prayer-time");
                slot.append(&n);
                slot.append(&t);
                strip.append(&slot);
            }
        })
    };

    for &faith in FaithTradition::ALL {
        let btn = Button::with_label(faith.pill_label());
        btn.add_css_class("umi-faith-pill");
        if cfg_init.prayer.faith == faith {
            btn.add_css_class("active");
        }
        let c = ctx.clone();
        let fbs = Rc::clone(&faith_btns);
        let rs = Rc::clone(&render_schedule);
        let sc = Rc::clone(&sync_custom_entry_text);
        btn.connect_clicked(move |_| {
            {
                let mut cfg = c.cfg.borrow_mut();
                if cfg.prayer.faith != faith {
                    cfg.prayer.faith = faith;
                    cfg.prayer.custom_slots.clear();
                }
            }
            for (fid, b) in fbs.borrow().iter() {
                if *fid == faith {
                    b.add_css_class("active");
                } else {
                    b.remove_css_class("active");
                }
            }
            c.commit();
            sc();
            rs();
        });
        faith_strip.append(&btn);
        faith_btns.borrow_mut().push((faith, btn));
    }

    page.append(&faith_strip);
    page.append(&sched_hdr);
    page.append(&sched_strip);
    page.append(&feast_banner);

    let card = GtkBox::new(Orientation::Vertical, 10);
    card.add_css_class("umi-card");

    // --- Islam-specific options (Location Source, City, Calculation Method) ---
    {
        let row = GtkBox::new(Orientation::Horizontal, 6);
        let lbl = Label::new(Some("Location Source"));
        lbl.add_css_class("umi-row-title");
        lbl.set_hexpand(true);
        lbl.set_xalign(0.0);
        row.append(&lbl);

        let b_auto = Button::with_label("My Location (Auto Wi-Fi)");
        b_auto.add_css_class("umi-seg-btn");
        let b_pick = Button::with_label("Pick City");
        b_pick.add_css_class("umi-seg-btn");
        if cfg_init.prayer.auto_location {
            b_auto.add_css_class("active");
        } else {
            b_pick.add_css_class("active");
        }
        {
            let c = ctx.clone();
            let ba = b_auto.clone();
            let bp = b_pick.clone();
            b_auto.connect_clicked(move |_| {
                c.cfg.borrow_mut().prayer.auto_location = true;
                ba.add_css_class("active");
                bp.remove_css_class("active");
                c.commit();
            });
        }
        {
            let c = ctx.clone();
            let ba = b_auto.clone();
            let bp = b_pick.clone();
            b_pick.connect_clicked(move |_| {
                c.cfg.borrow_mut().prayer.auto_location = false;
                bp.add_css_class("active");
                ba.remove_css_class("active");
                c.commit();
            });
        }
        row.append(&b_auto);
        row.append(&b_pick);
        islam_box.append(&row);
        islam_box.append(&make_sep());

        // City dropdown
        let row_c = GtkBox::new(Orientation::Horizontal, 8);
        let lbl_c = Label::new(Some("Detected / Fallback City"));
        lbl_c.add_css_class("umi-row-title");
        lbl_c.set_hexpand(true);
        lbl_c.set_xalign(0.0);
        let names: Vec<&str> = CITIES.iter().map(|(c, _, _)| *c).collect();
        let model = StringList::new(&names);
        let dd = DropDown::new(Some(model), None::<gtk4::Expression>);
        let init_idx = CITIES
            .iter()
            .position(|(c, _, _)| c.eq_ignore_ascii_case(&cfg_init.prayer.city))
            .unwrap_or(0) as u32;
        dd.set_selected(init_idx);
        {
            let c = ctx.clone();
            let rs = Rc::clone(&render_schedule);
            dd.connect_selected_notify(move |d| {
                if let Some((city, lat, lon)) = CITIES.get(d.selected() as usize) {
                    {
                        let mut cfg = c.cfg.borrow_mut();
                        cfg.prayer.city = (*city).to_string();
                        cfg.prayer.latitude = *lat;
                        cfg.prayer.longitude = *lon;
                    }
                    c.commit();
                    rs();
                }
            });
        }
        row_c.append(&lbl_c);
        row_c.append(&dd);
        islam_box.append(&row_c);
        islam_box.append(&make_sep());

        // Calculation Method dropdown
        let row_m = GtkBox::new(Orientation::Horizontal, 8);
        let lbl_m = Label::new(Some("Calculation Method"));
        lbl_m.add_css_class("umi-row-title");
        lbl_m.set_hexpand(true);
        lbl_m.set_xalign(0.0);
        let labels: Vec<&str> = METHODS.iter().map(|(_, l)| *l).collect();
        let model_m = StringList::new(&labels);
        let dd_m = DropDown::new(Some(model_m), None::<gtk4::Expression>);
        let init_m = METHODS
            .iter()
            .position(|(id, _)| *id == cfg_init.prayer.method)
            .unwrap_or(0) as u32;
        dd_m.set_selected(init_m);
        {
            let c = ctx.clone();
            let rs = Rc::clone(&render_schedule);
            dd_m.connect_selected_notify(move |d| {
                if let Some((id, _)) = METHODS.get(d.selected() as usize) {
                    c.cfg.borrow_mut().prayer.method = (*id).to_string();
                    c.commit();
                    rs();
                }
            });
        }
        row_m.append(&lbl_m);
        row_m.append(&dd_m);
        islam_box.append(&row_m);
    }

    // --- Non-Islam custom slots + lunar/weekly feast reminder ---
    {
        let row = GtkBox::new(Orientation::Horizontal, 8);
        let lbl = Label::new(Some("Custom Daily Times (HH:MM)"));
        lbl.add_css_class("umi-row-title");
        lbl.set_xalign(0.0);
        let save_btn = Button::with_label("Apply");
        save_btn.add_css_class("umi-action-btn");
        {
            let c = ctx.clone();
            let ce = custom_entry.clone();
            let rs = Rc::clone(&render_schedule);
            save_btn.connect_clicked(move |_| {
                let parts: Vec<String> = ce
                    .text()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                c.cfg.borrow_mut().prayer.custom_slots = parts;
                c.commit();
                rs();
            });
        }
        let reset_btn = Button::with_label("Default");
        reset_btn.add_css_class("umi-seg-btn");
        {
            let c = ctx.clone();
            let rs = Rc::clone(&render_schedule);
            let sc = Rc::clone(&sync_custom_entry_text);
            reset_btn.connect_clicked(move |_| {
                c.cfg.borrow_mut().prayer.custom_slots.clear();
                c.commit();
                sc();
                rs();
            });
        }
        row.append(&lbl);
        row.append(&custom_entry);
        row.append(&save_btn);
        row.append(&reset_btn);
        other_box.append(&row);
        other_box.append(&make_sep());

        let c_fr = ctx.clone();
        let rs_fr = Rc::clone(&render_schedule);
        other_box.append(&make_toggle_row(
            "Holy Day / Lunar & Sunday Reminder",
            "Auto-detect Purnama / Tilem / Uposatha / Che It & Cap Go / Sunday worship",
            cfg_init.prayer.show_feast_reminder,
            move |v| {
                c_fr.cfg.borrow_mut().prayer.show_feast_reminder = v;
                c_fr.commit();
                rs_fr();
            },
        ));
    }

    card.append(&islam_box);
    card.append(&other_box);
    card.append(&make_sep());

    // Pre-Prayer / Devotion Nudge
    {
        let row = GtkBox::new(Orientation::Horizontal, 6);
        let lbl = Label::new(Some("Pre-Prayer / Devotion Nudge"));
        lbl.add_css_class("umi-row-title");
        lbl.set_hexpand(true);
        lbl.set_xalign(0.0);
        row.append(&lbl);
        let n_btns: Rc<RefCell<Vec<(u32, Button)>>> = Rc::new(RefCell::new(Vec::new()));
        for mins in [5u32, 10, 15] {
            let b = Button::with_label(&format!("{mins} min"));
            b.add_css_class("umi-seg-btn");
            if cfg_init.prayer.pre_nudge_min == mins {
                b.add_css_class("active");
            }
            let c = ctx.clone();
            let nbs = Rc::clone(&n_btns);
            b.connect_clicked(move |_| {
                c.cfg.borrow_mut().prayer.pre_nudge_min = mins;
                for (nid, btn) in nbs.borrow().iter() {
                    if *nid == mins {
                        btn.add_css_class("active");
                    } else {
                        btn.remove_css_class("active");
                    }
                }
                c.commit();
            });
            row.append(&b);
            n_btns.borrow_mut().push((mins, b));
        }
        card.append(&row);
    }
    card.append(&make_sep());

    let c_ap = ctx.clone();
    card.append(&make_toggle_row(
        "Auto-Pause Media on Prayer / Devotion Time",
        "Automatically pause Spotify / YouTube when schedule arrives",
        cfg_init.prayer.auto_pause_music,
        move |v| {
            c_ap.cfg.borrow_mut().prayer.auto_pause_music = v;
            c_ap.commit();
        },
    ));
    card.append(&make_sep());

    let c_sa = ctx.clone();
    card.append(&make_toggle_row(
        "Show Sun Arc / Slot Progress in Notch",
        "Calculated 100% offline on your device without tracking",
        cfg_init.prayer.show_sun_arc,
        move |v| {
            c_sa.cfg.borrow_mut().prayer.show_sun_arc = v;
            c_sa.commit();
        },
    ));

    sync_custom_entry_text();
    render_schedule();

    page.append(&card);
    page
}

/// Screen 8 · Widgets & Calendar (FigCraft Settings 08 — 2-Slot Picker, 8 Widgets, Calendar To-Do)
fn build_screen_widgets_calendar(ctx: &SettingsCtx) -> GtkBox {
    let page = GtkBox::new(Orientation::Vertical, 12);

    // Header row + Swap L/R button
    let hdr_row = GtkBox::new(Orientation::Horizontal, 8);
    let hdr_left = page_header(
        "Widgets & Calendar",
        "Choose 2 mini-widgets for State 14 · Widgets + Calendar To-Do preferences.",
    );
    hdr_left.set_hexpand(true);
    let swap_btn = Button::with_label("⇄ Swap L/R");
    swap_btn.add_css_class("umi-action-btn");
    swap_btn.set_valign(Align::Center);
    hdr_row.append(&hdr_left);
    hdr_row.append(&swap_btn);
    page.append(&hdr_row);

    // Card 1: Active Notch Slots Preview (Left + Right)
    let preview_card = GtkBox::new(Orientation::Vertical, 8);
    preview_card.add_css_class("umi-card");
    preview_card.append(&section_label("ACTIVE NOTCH SLOTS · STATE 14 (2 SLOTS)"));

    let slots_row = GtkBox::new(Orientation::Horizontal, 10);
    slots_row.set_homogeneous(true);

    let left_box = GtkBox::new(Orientation::Vertical, 2);
    left_box.add_css_class("umi-slot-preview-left");
    let left_tag = Label::new(Some("LEFT SLOT"));
    left_tag.add_css_class("umi-slot-tag-left");
    left_tag.set_xalign(0.0);
    let left_name = Label::new(None);
    left_name.add_css_class("umi-row-title");
    left_name.set_xalign(0.0);
    left_box.append(&left_tag);
    left_box.append(&left_name);

    let right_box = GtkBox::new(Orientation::Vertical, 2);
    right_box.add_css_class("umi-slot-preview-right");
    let right_tag = Label::new(Some("RIGHT SLOT"));
    right_tag.add_css_class("umi-slot-tag-right");
    right_tag.set_xalign(0.0);
    let right_name = Label::new(None);
    right_name.add_css_class("umi-row-title");
    right_name.set_xalign(0.0);
    right_box.append(&right_tag);
    right_box.append(&right_name);

    slots_row.append(&left_box);
    slots_row.append(&right_box);
    preview_card.append(&slots_row);
    page.append(&preview_card);

    // Card 2: Widget Gallery (8 selectable widgets in 2x4 grid)
    let gallery_card = GtkBox::new(Orientation::Vertical, 8);
    gallery_card.add_css_class("umi-card");
    gallery_card.append(&section_label(
        "WIDGET GALLERY (8 PILIHAN) — UNUSED WIDGETS SLEEP AT 0% CPU",
    ));

    let widget_controls: Rc<RefCell<Vec<(WidgetKind, GtkBox, Button, Button)>>> =
        Rc::new(RefCell::new(Vec::new()));

    let sync_widgets_ui = {
        let cfg_rc = Rc::clone(&ctx.cfg);
        let ln = left_name.clone();
        let rn = right_name.clone();
        let ctrls = Rc::clone(&widget_controls);
        Rc::new(move || {
            let (wl, wr) = {
                let c = cfg_rc.borrow();
                (c.widgets.left, c.widgets.right)
            };
            ln.set_text(&format!("{}  {}", wl.glyph(), wl.title()));
            rn.set_text(&format!("{}  {}", wr.glyph(), wr.title()));
            for (kind, card, bl, br) in ctrls.borrow().iter() {
                card.remove_css_class("slot-left");
                card.remove_css_class("slot-right");
                bl.remove_css_class("active-left");
                br.remove_css_class("active-right");
                if *kind == wl {
                    card.add_css_class("slot-left");
                    bl.add_css_class("active-left");
                    bl.set_label("✓ L");
                } else {
                    bl.set_label("Kiri");
                }
                if *kind == wr {
                    card.add_css_class("slot-right");
                    br.add_css_class("active-right");
                    br.set_label("✓ R");
                } else {
                    br.set_label("Kanan");
                }
            }
        })
    };

    for chunk in WidgetKind::ALL.chunks(2) {
        let row = GtkBox::new(Orientation::Horizontal, 8);
        row.set_homogeneous(true);
        for &kind in chunk {
            let wcard = GtkBox::new(Orientation::Horizontal, 6);
            wcard.add_css_class("umi-widget-card");
            wcard.set_valign(Align::Center);

            let info = GtkBox::new(Orientation::Vertical, 1);
            info.set_hexpand(true);
            let t = Label::new(Some(&format!("{}  {}", kind.glyph(), kind.title())));
            t.add_css_class("umi-row-title");
            t.set_xalign(0.0);
            let s = Label::new(Some(kind.subtitle()));
            s.add_css_class("umi-row-sub");
            s.set_xalign(0.0);
            info.append(&t);
            info.append(&s);

            let bl = Button::with_label("Kiri");
            bl.add_css_class("umi-slot-btn");
            bl.set_valign(Align::Center);
            let br = Button::with_label("Kanan");
            br.add_css_class("umi-slot-btn");
            br.set_valign(Align::Center);

            {
                let c = ctx.clone();
                let sync = Rc::clone(&sync_widgets_ui);
                bl.connect_clicked(move |_| {
                    c.cfg.borrow_mut().widgets.left = kind;
                    c.commit();
                    sync();
                });
            }
            {
                let c = ctx.clone();
                let sync = Rc::clone(&sync_widgets_ui);
                br.connect_clicked(move |_| {
                    c.cfg.borrow_mut().widgets.right = kind;
                    c.commit();
                    sync();
                });
            }

            wcard.append(&info);
            wcard.append(&bl);
            wcard.append(&br);
            row.append(&wcard);
            widget_controls.borrow_mut().push((kind, wcard, bl, br));
        }
        gallery_card.append(&row);
    }

    {
        let c = ctx.clone();
        let sync = Rc::clone(&sync_widgets_ui);
        swap_btn.connect_clicked(move |_| {
            {
                let mut cfg = c.cfg.borrow_mut();
                let w = &mut cfg.widgets;
                std::mem::swap(&mut w.left, &mut w.right);
            }
            c.commit();
            sync();
        });
    }

    sync_widgets_ui();
    page.append(&gallery_card);

    // Card 3: Calendar To-Do, Sensor Refresh & Quick Notes
    let cfg_init = ctx.cfg.borrow().clone();
    let cal_card = GtkBox::new(Orientation::Vertical, 10);
    cal_card.add_css_class("umi-card");
    cal_card.append(&section_label("CALENDAR TO-DO, QUICK NOTES & SENSOR REFRESH"));

    // Week starts on Monday / Sunday
    {
        let row = GtkBox::new(Orientation::Horizontal, 6);
        let lbl = Label::new(Some("Calendar Week Starts On"));
        lbl.add_css_class("umi-row-title");
        lbl.set_hexpand(true);
        lbl.set_xalign(0.0);
        row.append(&lbl);

        let b_mon = Button::with_label("Monday");
        b_mon.add_css_class("umi-seg-btn");
        let b_sun = Button::with_label("Sunday");
        b_sun.add_css_class("umi-seg-btn");
        if cfg_init.widgets.week_starts_monday {
            b_mon.add_css_class("active");
        } else {
            b_sun.add_css_class("active");
        }
        {
            let c = ctx.clone();
            let bm = b_mon.clone();
            let bs = b_sun.clone();
            b_mon.connect_clicked(move |_| {
                c.cfg.borrow_mut().widgets.week_starts_monday = true;
                bm.add_css_class("active");
                bs.remove_css_class("active");
                c.commit();
            });
        }
        {
            let c = ctx.clone();
            let bm = b_mon.clone();
            let bs = b_sun.clone();
            b_sun.connect_clicked(move |_| {
                c.cfg.borrow_mut().widgets.week_starts_monday = false;
                bs.add_css_class("active");
                bm.remove_css_class("active");
                c.commit();
            });
        }
        row.append(&b_mon);
        row.append(&b_sun);
        cal_card.append(&row);
    }
    cal_card.append(&make_sep());

    let c_cal = ctx.clone();
    cal_card.append(&make_toggle_row(
        "Show Date Picker & Task Dots in To-Do (State 13)",
        "Klik tanggal mana saja di kalender untuk lihat & tulis What to Do hari itu",
        cfg_init.widgets.show_todo_calendar,
        move |v| {
            c_cal.cfg.borrow_mut().widgets.show_todo_calendar = v;
            c_cal.commit();
        },
    ));
    cal_card.append(&make_sep());

    // Sensor Refresh Interval (1s / 2s Hemat / 5s)
    {
        let row = GtkBox::new(Orientation::Horizontal, 6);
        let lbl = Label::new(Some("Sensor Refresh Interval (CPU / RAM / Network)"));
        lbl.add_css_class("umi-row-title");
        lbl.set_hexpand(true);
        lbl.set_xalign(0.0);
        row.append(&lbl);

        let r_btns: Rc<RefCell<Vec<(u32, Button)>>> = Rc::new(RefCell::new(Vec::new()));
        for (secs, text) in [(1u32, "1s"), (2, "2s Hemat"), (5, "5s")] {
            let b = Button::with_label(text);
            b.add_css_class("umi-seg-btn");
            if cfg_init.widgets.refresh_secs == secs {
                b.add_css_class("active");
            }
            let c = ctx.clone();
            let rbs = Rc::clone(&r_btns);
            b.connect_clicked(move |_| {
                c.cfg.borrow_mut().widgets.refresh_secs = secs;
                for (sid, btn) in rbs.borrow().iter() {
                    if *sid == secs {
                        btn.add_css_class("active");
                    } else {
                        btn.remove_css_class("active");
                    }
                }
                c.commit();
            });
            row.append(&b);
            r_btns.borrow_mut().push((secs, b));
        }
        cal_card.append(&row);
    }
    cal_card.append(&make_sep());

    // Quick Notes editor (persisted to SQLite)
    {
        let row = GtkBox::new(Orientation::Horizontal, 8);
        let lbl = Label::new(Some("Quick Notes Text"));
        lbl.add_css_class("umi-row-title");
        lbl.set_xalign(0.0);
        let note_entry = Entry::new();
        note_entry.add_css_class("umi-settings-entry");
        note_entry.set_hexpand(true);
        note_entry.set_placeholder_text(Some("Tulis catatan singkat untuk widget Quick Notes..."));
        if let Some(ref db) = ctx.db {
            if let Ok(cur) = db.get_quick_note() {
                note_entry.set_text(&cur);
            }
        }
        let save_note_btn = Button::with_label("Save");
        save_note_btn.add_css_class("umi-action-btn");
        {
            let c = ctx.clone();
            let ne = note_entry.clone();
            save_note_btn.connect_clicked(move |_| {
                if let Some(ref db) = c.db {
                    let _ = db.set_quick_note(&ne.text());
                }
                c.commit();
            });
        }
        row.append(&lbl);
        row.append(&note_entry);
        row.append(&save_note_btn);
        cal_card.append(&row);
    }

    page.append(&cal_card);
    page
}

/// Screen 6 · AI Assistant & Global Shortcuts
fn build_screen_ai(ctx: &SettingsCtx) -> GtkBox {
    let page = GtkBox::new(Orientation::Vertical, 10);
    page.append(&page_header(
        "AI Assistant & Global Shortcuts",
        "Manage BYOK API keys (KWallet encrypted), AI Quota scripts, and global hotkeys.",
    ));

    let cfg_init = ctx.cfg.borrow().clone();

    page.append(&section_label("AI ASSISTANT & KWALLET BYOK"));
    let ai_card = GtkBox::new(Orientation::Vertical, 10);
    ai_card.add_css_class("umi-card");

    // Provider segmented selector
    let prov_row = GtkBox::new(Orientation::Horizontal, 6);
    let prov_lbl = Label::new(Some("AI Provider"));
    prov_lbl.add_css_class("umi-row-title");
    prov_lbl.set_hexpand(true);
    prov_lbl.set_xalign(0.0);
    prov_row.append(&prov_lbl);
    let p_btns: Rc<RefCell<Vec<(String, Button)>>> = Rc::new(RefCell::new(Vec::new()));
    let key_status = Label::new(Some("Checking KWallet..."));
    key_status.add_css_class("umi-warn-pill");

    let check_kwallet_status = {
        let rt_h = ctx.rt_handle.clone();
        let status_lbl = key_status.clone();
        Rc::new(move |provider: String| {
            if provider == "ollama" {
                status_lbl.set_text("Local · No key needed");
                status_lbl.remove_css_class("umi-warn-pill");
                status_lbl.add_css_class("umi-verified-pill");
                return;
            }
            let (tx, rx) = async_channel::bounded::<bool>(1);
            let p_clone = provider.clone();
            rt_h.spawn(async move {
                let ok = secrets::read_secret(&p_clone).await.is_some();
                let _ = tx.send(ok).await;
            });
            let sl = status_lbl.clone();
            glib::spawn_future_local(async move {
                if let Ok(has_key) = rx.recv().await {
                    if has_key {
                        sl.set_text("✓ Saved in KWallet");
                        sl.remove_css_class("umi-warn-pill");
                        sl.add_css_class("umi-verified-pill");
                    } else {
                        sl.set_text("Key not set");
                        sl.remove_css_class("umi-verified-pill");
                        sl.add_css_class("umi-warn-pill");
                    }
                }
            });
        })
    };
    check_kwallet_status(cfg_init.ai.provider.clone());

    for (pid, name) in [
        ("gemini", "Gemini (BYOK)"),
        ("claude", "Claude"),
        ("openai", "OpenAI"),
        ("ollama", "Ollama Local"),
    ] {
        let b = Button::with_label(name);
        b.add_css_class("umi-seg-btn");
        if cfg_init.ai.provider == pid {
            b.add_css_class("active");
        }
        let c = ctx.clone();
        let pbs = Rc::clone(&p_btns);
        let ck = Rc::clone(&check_kwallet_status);
        let pid_s = pid.to_string();
        b.connect_clicked(move |_| {
            c.cfg.borrow_mut().ai.provider = pid_s.clone();
            for (id, btn) in pbs.borrow().iter() {
                if id == &pid_s {
                    btn.add_css_class("active");
                } else {
                    btn.remove_css_class("active");
                }
            }
            c.commit();
            ck(pid_s.clone());
        });
        prov_row.append(&b);
        p_btns.borrow_mut().push((pid.to_string(), b));
    }
    ai_card.append(&prov_row);
    ai_card.append(&make_sep());

    // Encrypted API Key input + Save to KWallet
    let key_row = GtkBox::new(Orientation::Horizontal, 8);
    let key_lbl = Label::new(Some("API Key (KWallet)"));
    key_lbl.add_css_class("umi-row-title");
    key_lbl.set_width_request(125);
    key_lbl.set_xalign(0.0);
    let key_entry = PasswordEntry::new();
    key_entry.set_show_peek_icon(true);
    key_entry.set_placeholder_text(Some("Paste API key → saved encrypted in KDE Wallet"));
    key_entry.set_hexpand(true);
    key_entry.add_css_class("umi-settings-entry");

    let save_key_btn = Button::with_label("Save Key");
    save_key_btn.add_css_class("umi-action-btn");
    {
        let c = ctx.clone();
        let ke = key_entry.clone();
        let sl = key_status.clone();
        save_key_btn.connect_clicked(move |_| {
            let secret = ke.text().to_string();
            if secret.trim().is_empty() {
                return;
            }
            let provider = c.cfg.borrow().ai.provider.clone();
            let (tx, rx) = async_channel::bounded::<Result<(), String>>(1);
            c.rt_handle.spawn(async move {
                let res = secrets::save_secret(&provider, &secret)
                    .await
                    .map_err(|e| e.to_string());
                let _ = tx.send(res).await;
            });
            let ke2 = ke.clone();
            let sl2 = sl.clone();
            glib::spawn_future_local(async move {
                if let Ok(res) = rx.recv().await {
                    match res {
                        Ok(()) => {
                            ke2.set_text("");
                            sl2.set_text("✓ Saved in KWallet");
                            sl2.remove_css_class("umi-warn-pill");
                            sl2.add_css_class("umi-verified-pill");
                        }
                        Err(e) => {
                            sl2.set_text(&format!("Error: {}", e.chars().take(24).collect::<String>()));
                            sl2.remove_css_class("umi-verified-pill");
                            sl2.add_css_class("umi-warn-pill");
                        }
                    }
                }
            });
        });
    }
    key_row.append(&key_lbl);
    key_row.append(&key_entry);
    key_row.append(&save_key_btn);
    key_row.append(&key_status);
    ai_card.append(&key_row);
    ai_card.append(&make_sep());

    // Gemini Model & Quota scripts folder
    let model_row = GtkBox::new(Orientation::Horizontal, 8);
    let m_lbl = Label::new(Some("Gemini Model"));
    m_lbl.add_css_class("umi-row-title");
    m_lbl.set_width_request(125);
    m_lbl.set_xalign(0.0);
    let model_entry = Entry::new();
    model_entry.set_text(&cfg_init.ai.gemini_model);
    model_entry.set_hexpand(true);
    model_entry.add_css_class("umi-settings-entry");
    {
        let c = ctx.clone();
        model_entry.connect_changed(move |e| {
            let txt = e.text().trim().to_string();
            if !txt.is_empty() {
                c.cfg.borrow_mut().ai.gemini_model = txt;
                c.commit();
            }
        });
    }
    let scripts_btn = Button::with_label("Quota Scripts Folder");
    scripts_btn.add_css_class("umi-seg-btn");
    scripts_btn.connect_clicked(|_| {
        if let Some(base) = BaseDirs::new() {
            let p = base.config_dir().join("umi/quota-scripts");
            let _ = std::fs::create_dir_all(&p);
            let _ = std::process::Command::new("xdg-open").arg(p).spawn();
        }
    });
    model_row.append(&m_lbl);
    model_row.append(&model_entry);
    model_row.append(&scripts_btn);
    ai_card.append(&model_row);
    ai_card.append(&make_sep());

    // Tracked AI Subscriptions (only show providers the user actually subscribes to)
    let track_row = GtkBox::new(Orientation::Horizontal, 6);
    let track_lbl = Label::new(Some("AI Limits Tracked"));
    track_lbl.add_css_class("umi-row-title");
    track_lbl.set_hexpand(true);
    track_lbl.set_xalign(0.0);
    track_row.append(&track_lbl);
    for (qid, qname) in [
        ("antigravity", "Antigravity"),
        ("codex", "Codex"),
        ("claude", "Claude"),
    ] {
        let b = Button::with_label(qname);
        b.add_css_class("umi-seg-btn");
        if cfg_init
            .ai
            .quota_providers
            .iter()
            .any(|p| p.eq_ignore_ascii_case(qid))
        {
            b.add_css_class("active");
        }
        let c = ctx.clone();
        let qid_s = qid.to_string();
        let b_ref = b.clone();
        b.connect_clicked(move |_| {
            let now_on = {
                let mut cfg = c.cfg.borrow_mut();
                if let Some(pos) = cfg
                    .ai
                    .quota_providers
                    .iter()
                    .position(|p| p.eq_ignore_ascii_case(&qid_s))
                {
                    cfg.ai.quota_providers.remove(pos);
                    false
                } else {
                    cfg.ai.quota_providers.push(qid_s.clone());
                    true
                }
            };
            if now_on {
                b_ref.add_css_class("active");
            } else {
                b_ref.remove_css_class("active");
            }
            c.commit();
        });
        track_row.append(&b);
    }
    ai_card.append(&track_row);
    page.append(&ai_card);

    // Shortcuts table
    page.append(&section_label("GLOBAL SHORTCUTS (KDE WAYLAND PORTAL)"));
    let sc_card = GtkBox::new(Orientation::Vertical, 9);
    sc_card.add_css_class("umi-card");

    let shortcuts: &[(&str, &str, &[&str])] = &[
        ("Ask UMI Overlay", "Open AI prompt directly inside the notch", &["Ctrl", "Alt", "Space"]),
        ("Translate Mode", "Press while inside Ask UMI to toggle translation", &["Tab"]),
        ("Smart Screen OCR", "Capture region on screen → Gemini Vision OCR", &["Ctrl", "Alt", "O"]),
        ("Toggle UMI Notch", "Expand or collapse notch from any workspace", &["Meta", "N"]),
        ("Quick Math & Currency", "Type 100usd to idr= in Ask UMI to convert live", &["="]),
        ("Open UMI Settings", "Open this preferences window while notch focused", &["Ctrl", ","]),
    ];

    for (idx, (title, sub, keys)) in shortcuts.iter().enumerate() {
        if idx > 0 {
            sc_card.append(&make_sep());
        }
        let r = GtkBox::new(Orientation::Horizontal, 8);
        let left = GtkBox::new(Orientation::Vertical, 1);
        left.set_hexpand(true);
        let t = Label::new(Some(title));
        t.add_css_class("umi-row-title");
        t.set_xalign(0.0);
        let s = Label::new(Some(sub));
        s.add_css_class("umi-row-sub");
        s.set_xalign(0.0);
        left.append(&t);
        left.append(&s);
        r.append(&left);
        for k in *keys {
            let cap = Label::new(Some(k));
            cap.add_css_class("umi-keycap");
            r.append(&cap);
        }
        sc_card.append(&r);
    }
    page.append(&sc_card);
    page
}

/// Screen 7 · About UMI & Local Data
fn build_screen_about(ctx: &SettingsCtx) -> GtkBox {
    let page = GtkBox::new(Orientation::Vertical, 12);
    page.append(&page_header(
        "About UMI & Local Data",
        "Version details, offline privacy guarantees, backups, and credits.",
    ));

    // Hero app card
    let hero = GtkBox::new(Orientation::Horizontal, 16);
    hero.add_css_class("umi-card");
    hero.set_valign(Align::Center);
    hero.append(&make_logo_widget(68));

    let info = GtkBox::new(Orientation::Vertical, 4);
    info.set_hexpand(true);
    let app_name = Label::new(Some("UMI Desktop"));
    app_name.add_css_class("umi-page-title");
    app_name.set_xalign(0.0);
    let sub = Label::new(Some("Ambient Dynamic Island · Pure Rust GTK4 Wayland (Fedora KDE)"));
    sub.add_css_class("umi-row-sub");
    sub.set_xalign(0.0);
    info.append(&app_name);
    info.append(&sub);
    hero.append(&info);
    page.append(&hero);

    // Local storage summary
    page.append(&section_label("LOCAL STORAGE · NO ACCOUNT OR TELEMETRY"));
    let data_card = GtkBox::new(Orientation::Vertical, 10);
    data_card.add_css_class("umi-card");

    let (task_n, shelf_n, water_ml) = if let Some(ref db) = ctx.db {
        (
            db.list_tasks().map(|v| v.len()).unwrap_or(0),
            db.list_shelf_items().map(|v| v.len()).unwrap_or(0),
            db.today_water_summary().map(|w| w.today_ml).unwrap_or(0),
        )
    } else {
        (0, 0, 0)
    };

    for (idx, (lbl, val)) in [
        ("To-do List & Reminders", format!("{task_n} tasks · SQLite local")),
        (
            "Hydration Log & Shelf Paths",
            format!("{water_ml} ml today · {shelf_n} files"),
        ),
        (
            "Clipboard History & API Keys",
            "Klipper sync · KWallet encrypted".to_string(),
        ),
    ]
    .iter()
    .enumerate()
    {
        if idx > 0 {
            data_card.append(&make_sep());
        }
        let r = GtkBox::new(Orientation::Horizontal, 8);
        let l = Label::new(Some(lbl));
        l.add_css_class("umi-row-title");
        l.set_hexpand(true);
        l.set_xalign(0.0);
        let v = Label::new(Some(val));
        v.add_css_class("umi-chip");
        r.append(&l);
        r.append(&v);
        data_card.append(&r);
    }

    let status_lbl = Label::new(Some(""));
    status_lbl.add_css_class("umi-row-sub");
    status_lbl.set_xalign(0.0);

    let act_row = GtkBox::new(Orientation::Horizontal, 8);
    act_row.set_margin_top(6);

    let export_btn = Button::with_label("Export Settings JSON");
    export_btn.add_css_class("umi-action-btn");
    {
        let c = ctx.clone();
        let st = status_lbl.clone();
        export_btn.connect_clicked(move |_| match desktop::export_settings_json(&c.cfg.borrow()) {
            Ok(path) => st.set_text(&format!("Exported to {}", path.display())),
            Err(e) => st.set_text(&format!("Export failed: {e}")),
        });
    }

    let folder_btn = Button::with_label("Open Data Folder");
    folder_btn.add_css_class("umi-seg-btn");
    folder_btn.connect_clicked(|_| {
        if let Some(base) = BaseDirs::new() {
            let p = base.config_dir().join("umi");
            let _ = std::fs::create_dir_all(&p);
            let _ = std::process::Command::new("xdg-open").arg(p).spawn();
        }
    });

    let reset_btn = Button::with_label("Reset Settings");
    reset_btn.add_css_class("umi-danger-btn");
    {
        let c = ctx.clone();
        let st = status_lbl.clone();
        reset_btn.connect_clicked(move |_| {
            *c.cfg.borrow_mut() = UmiConfig::default();
            c.commit();
            st.set_text("Settings reset to defaults.");
        });
    }

    act_row.append(&export_btn);
    act_row.append(&folder_btn);
    act_row.append(&reset_btn);
    data_card.append(&act_row);
    data_card.append(&status_lbl);
    page.append(&data_card);

    // Credits card
    let cred_card = GtkBox::new(Orientation::Vertical, 4);
    cred_card.add_css_class("umi-card");
    let ct = Label::new(Some("Data Sources & Credits"));
    ct.add_css_class("umi-row-title");
    ct.set_xalign(0.0);
    let cb = Label::new(Some(
        "Synced lyrics provided by LRCLIB. Wi-Fi location via BeaconDB & OpenStreetMap Nominatim. Prayer times & timers run 100% locally offline.",
    ));
    cb.add_css_class("umi-row-sub");
    cb.set_xalign(0.0);
    cb.set_wrap(true);
    cred_card.append(&ct);
    cred_card.append(&cb);
    page.append(&cred_card);

    page
}

/// Cairo-drawn UMI Tapered Notch Light App Tile (matches design.figcraft Page UMI Logo).
fn make_logo_widget(size: i32) -> DrawingArea {
    let da = DrawingArea::new();
    da.set_content_width(size);
    da.set_content_height(size);
    da.set_valign(Align::Center);
    da.set_draw_func(move |_, cr, w, h| {
        draw_umi_logo_tile(cr, w.min(h) as f64);
    });
    da
}

fn draw_umi_logo_tile(cr: &Context, s: f64) {
    let k = s / 220.0;
    let _ = cr.save();
    cr.scale(k, k);

    // Light App Tile (#F6F9FF -> #DCE6FF, rx=55 on 220x220)
    rrect(cr, 0.0, 0.0, 220.0, 220.0, 55.0);
    let bg = LinearGradient::new(44.0, 0.0, 176.0, 220.0);
    bg.add_color_stop_rgb(0.0, 0.965, 0.976, 1.0);
    bg.add_color_stop_rgb(1.0, 0.863, 0.902, 1.0);
    let _ = cr.set_source(&bg);
    let _ = cr.fill();

    // Exact Tapered Notch silhouette from assets/icons/umi-logo.svg (translated 26, 58)
    let trace_mark = |cr: &Context| {
        let _ = cr.save();
        cr.translate(26.0, 58.0);
        cr.new_sub_path();
        cr.move_to(6.72, 0.0);
        cr.line_to(161.28, 0.0);
        cr.curve_to(165.48, 0.0, 168.0, 2.53, 168.0, 5.91);
        cr.curve_to(168.0, 9.28, 164.64, 11.81, 159.6, 11.81);
        cr.curve_to(149.52, 11.81, 146.16, 18.56, 146.16, 30.38);
        cr.line_to(146.16, 59.06);
        cr.curve_to(146.16, 91.13, 124.32, 108.0, 84.0, 108.0);
        cr.curve_to(43.68, 108.0, 21.84, 91.13, 21.84, 59.06);
        cr.line_to(21.84, 30.38);
        cr.curve_to(21.84, 18.56, 18.48, 11.81, 8.4, 11.81);
        cr.curve_to(3.36, 11.81, 0.0, 9.28, 0.0, 5.91);
        cr.curve_to(0.0, 2.53, 2.52, 0.0, 6.72, 0.0);
        cr.close_path();
        let _ = cr.restore();
    };

    trace_mark(cr);
    cr.set_source_rgb(0.020, 0.027, 0.055); // #05070E
    let _ = cr.fill();

    // Rim gradient #38BDF8 -> #6366F1 -> #A855F7
    let rim = LinearGradient::new(26.0, 112.0, 194.0, 112.0);
    rim.add_color_stop_rgb(0.0, 0.220, 0.741, 0.973);
    rim.add_color_stop_rgb(0.5, 0.388, 0.400, 0.945);
    rim.add_color_stop_rgb(1.0, 0.659, 0.333, 0.969);
    trace_mark(cr);
    let _ = cr.set_source(&rim);
    cr.set_line_width(3.4);
    let _ = cr.stroke();

    // Squircle mascot eyes with #7DD3FC sky glint
    for (ex, gx) in [(76.0_f64, 83.25_f64), (124.0_f64, 131.25_f64)] {
        rrect(cr, ex, 95.0, 20.0, 25.0, 7.5);
        cr.set_source_rgb(1.0, 1.0, 1.0);
        let _ = cr.fill();
        cr.new_sub_path();
        cr.arc(gx, 102.25, 2.75, 0.0, 2.0 * PI);
        cr.set_source_rgb(0.490, 0.827, 0.988);
        let _ = cr.fill();
    }

    // Micro-smile (#FFFFFF99)
    cr.new_sub_path();
    cr.move_to(99.0, 128.0);
    cr.curve_to(106.33, 137.33, 113.67, 137.33, 121.0, 128.0);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.62);
    cr.set_line_width(2.8);
    cr.set_line_cap(cairo::LineCap::Round);
    let _ = cr.stroke();

    let _ = cr.restore();
}

/// Interactive Live Notch Preview inside Screen 1 (General)
#[allow(clippy::too_many_arguments)]
fn draw_live_notch_preview(
    cr: &Context,
    w: f64,
    h: f64,
    mode: u8,
    aura_rgb: [(f64, f64, f64); 3],
    eye_rgb: (f64, f64, f64),
    float_mix: f64,
    ear_mix: f64,
    glass_mix: f64,
    nw: f64,
    nh: f64,
) {
    let br = (nh * 0.44).min(16.0);
    let nx = (w - nw) * 0.5;
    let ny = 8.0 * float_mix;
    let eff_ear = 11.0 * ear_mix * (1.0 - float_mix * 2.0).clamp(0.0, 1.0);
    let eff_tr = br.min(nh * 0.5) * ((float_mix - 0.18) / 0.82).clamp(0.0, 1.0);

    let trace_silhouette = |cr: &Context| {
        cr.new_sub_path();
        if eff_ear <= 0.4 || eff_tr > 0.4 {
            if eff_tr > 0.4 {
                cr.arc(nx + eff_tr, ny + eff_tr, eff_tr, PI, PI * 1.5);
                cr.arc(nx + nw - eff_tr, ny + eff_tr, eff_tr, -PI * 0.5, 0.0);
            } else {
                cr.move_to(nx, ny);
                cr.line_to(nx + nw, ny);
            }
            cr.arc(nx + nw - br, ny + nh - br, br, 0.0, PI * 0.5);
            cr.arc(nx + br, ny + nh - br, br, PI * 0.5, PI);
            cr.close_path();
        } else {
            cr.move_to(nx - eff_ear, ny);
            cr.arc(nx - eff_ear, ny + eff_ear, eff_ear, -PI * 0.5, 0.0);
            cr.line_to(nx, ny + nh - br);
            cr.arc_negative(nx + br, ny + nh - br, br, PI, PI * 0.5);
            cr.line_to(nx + nw - br, ny + nh);
            cr.arc_negative(nx + nw - br, ny + nh - br, br, PI * 0.5, 0.0);
            cr.line_to(nx + nw, ny + eff_ear);
            cr.arc(nx + nw + eff_ear, ny + eff_ear, eff_ear, PI, PI * 1.5);
            cr.close_path();
        }
    };

    trace_silhouette(cr);
    let base_r = 0.065 * glass_mix;
    let base_g = 0.088 * glass_mix;
    let base_b = 0.148 * glass_mix;
    let alpha = 1.0 - 0.52 * glass_mix;
    cr.set_source_rgba(base_r, base_g, base_b, alpha);
    let _ = cr.fill();

    if glass_mix > 0.01 {
        let frost = LinearGradient::new(nx, ny, nx, ny + nh);
        frost.add_color_stop_rgba(0.0, 1.0, 1.0, 1.0, 0.22 * glass_mix);
        frost.add_color_stop_rgba(0.5, 0.85, 0.92, 1.0, 0.07 * glass_mix);
        frost.add_color_stop_rgba(1.0, 0.78, 0.88, 1.0, 0.14 * glass_mix);
        trace_silhouette(cr);
        let _ = cr.set_source(&frost);
        let _ = cr.fill();

        trace_silhouette(cr);
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.36 * glass_mix);
        cr.set_line_width(1.2);
        let _ = cr.stroke();
    }

    let [c0, c1, c2] = aura_rgb;

    if mode == 2 || nh < 18.0 {
        let g = LinearGradient::new(nx, ny, nx + nw, ny);
        g.add_color_stop_rgba(0.0, c0.0, c0.1, c0.2, 0.65);
        g.add_color_stop_rgba(0.5, c1.0, c1.1, c1.2, 0.85);
        g.add_color_stop_rgba(1.0, c2.0, c2.1, c2.2, 0.65);
        let _ = cr.set_source(&g);
        rrect(cr, nx + 6.0, ny + nh - 2.5, nw - 12.0, 2.5, 1.25);
        let _ = cr.fill();
        return;
    }

    if mode == 0 {
        // Resting mascot eyes + aura glow
        let g = LinearGradient::new(nx + 12.0, ny + nh, nx + nw - 12.0, ny + nh);
        g.add_color_stop_rgba(0.0, c0.0, c0.1, c0.2, 0.55);
        g.add_color_stop_rgba(0.5, c1.0, c1.1, c1.2, 0.70);
        g.add_color_stop_rgba(1.0, c2.0, c2.1, c2.2, 0.55);
        let _ = cr.set_source(&g);
        rrect(cr, nx + 18.0, ny + nh - 4.0, nw - 36.0, 3.0, 1.5);
        let _ = cr.fill();

        let (gr, gg, gb) = eye_rgb;
        for ex in [nx + nw * 0.5 - 21.0, nx + nw * 0.5 + 10.0] {
            rrect(cr, ex, ny + 10.0, 11.0, 13.0, 4.0);
            cr.set_source_rgb(1.0, 1.0, 1.0);
            let _ = cr.fill();
            cr.new_sub_path();
            cr.arc(ex + 7.5, ny + 14.2, 2.2, 0.0, 2.0 * PI);
            cr.set_source_rgb(gr, gg, gb);
            let _ = cr.fill();
        }
        cr.new_sub_path();
        cr.arc(nx + nw * 0.5, ny + 25.0, 4.2, 0.25 * PI, 0.75 * PI);
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.55);
        cr.set_line_width(1.4);
        let _ = cr.stroke();
    } else {
        // Closed: battery ring + time + EQ bars
        cr.new_sub_path();
        cr.arc(nx + 24.0, ny + nh * 0.5, 6.0, 0.0, 2.0 * PI);
        cr.set_source_rgba(0.18, 0.86, 0.42, 0.9);
        cr.set_line_width(2.0);
        let _ = cr.stroke();

        cr.select_font_face("Inter", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        cr.set_font_size(12.5);
        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.move_to(nx + nw * 0.5 - 18.0, ny + nh * 0.5 + 4.5);
        let _ = cr.show_text("17:45");

        for (i, bh) in [6.0, 11.0, 8.0, 12.0, 5.0].iter().enumerate() {
            let bx = nx + nw - 34.0 + i as f64 * 4.0;
            rrect(cr, bx, ny + (nh - bh) * 0.5, 2.2, *bh, 1.1);
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.85);
            let _ = cr.fill();
        }
    }
    let _ = h;
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

fn parse_hex(hex: &str) -> (f64, f64, f64) {
    let h = hex.trim_start_matches('#');
    if h.len() == 6 {
        let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(56) as f64 / 255.0;
        let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(189) as f64 / 255.0;
        let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(248) as f64 / 255.0;
        return (r, g, b);
    }
    (0.220, 0.741, 0.973)
}

fn load_settings_css() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let provider = CssProvider::new();
        provider.load_from_string(include_str!("../../assets/styles/settings.css"));
        if let Some(display) = gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_USER,
            );
        }
    });
}
