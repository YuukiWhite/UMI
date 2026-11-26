use crate::state::AppEvent;
use ksni::menu::{CheckmarkItem, StandardItem};
use ksni::{Icon, MenuItem, Tray, TrayService};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct TrayFlags {
    pub start_at_login: Arc<AtomicBool>,
    pub mascot_eyes: Arc<AtomicBool>,
    pub minimal_mode: Arc<AtomicBool>,
}

struct UmiTray {
    tx: async_channel::Sender<AppEvent>,
    flags: TrayFlags,
    icon_pixmap: Vec<Icon>,
}

impl UmiTray {
    fn send(&self, ev: AppEvent) {
        let _ = self.tx.try_send(ev);
    }
}

impl Tray for UmiTray {
    fn id(&self) -> String {
        "org.umi.Notch".into()
    }

    fn title(&self) -> String {
        "UMI — Ambient Notch Companion".into()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        self.icon_pixmap.clone()
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        self.send(AppEvent::ToggleNotch);
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            MenuItem::Standard(StandardItem {
                label: "UMI · Active".into(),
                enabled: false,
                ..Default::default()
            }),
            MenuItem::Separator,
            MenuItem::Checkmark(CheckmarkItem {
                label: "Start at Login".into(),
                checked: self.flags.start_at_login.load(Ordering::Relaxed),
                activate: Box::new(|this: &mut Self| {
                    let prev = this.flags.start_at_login.load(Ordering::Relaxed);
                    this.flags.start_at_login.store(!prev, Ordering::Relaxed);
                    this.send(AppEvent::ToggleStartAtLogin);
                }),
                ..Default::default()
            }),
            MenuItem::Checkmark(CheckmarkItem {
                label: "UMI Mascot".into(),
                checked: self.flags.mascot_eyes.load(Ordering::Relaxed),
                activate: Box::new(|this: &mut Self| {
                    let prev = this.flags.mascot_eyes.load(Ordering::Relaxed);
                    this.flags.mascot_eyes.store(!prev, Ordering::Relaxed);
                    this.send(AppEvent::ToggleMascotEyes);
                }),
                ..Default::default()
            }),
            MenuItem::Checkmark(CheckmarkItem {
                label: "Minimal notch".into(),
                checked: self.flags.minimal_mode.load(Ordering::Relaxed),
                activate: Box::new(|this: &mut Self| {
                    let prev = this.flags.minimal_mode.load(Ordering::Relaxed);
                    this.flags.minimal_mode.store(!prev, Ordering::Relaxed);
                    this.send(AppEvent::ToggleMinimalMode);
                }),
                ..Default::default()
            }),
            MenuItem::Separator,
            MenuItem::Standard(StandardItem {
                label: "Toggle Notch".into(),
                activate: Box::new(|this: &mut Self| this.send(AppEvent::ToggleNotch)),
                ..Default::default()
            }),
            MenuItem::Standard(StandardItem {
                label: "Settings...".into(),
                activate: Box::new(|this: &mut Self| this.send(AppEvent::OpenSettings)),
                ..Default::default()
            }),
            MenuItem::Standard(StandardItem {
                label: "Exit".into(),
                activate: Box::new(|this: &mut Self| this.send(AppEvent::Quit)),
                ..Default::default()
            }),
        ]
    }
}

/// Spawn KDE StatusNotifierItem tray service on a background thread.
pub fn spawn_tray(tx: async_channel::Sender<AppEvent>, flags: TrayFlags) -> ksni::Handle<impl Tray> {
    let icon = render_umi_tray_icon(32);
    let service = TrayService::new(UmiTray {
        tx,
        flags,
        icon_pixmap: vec![icon],
    });
    let handle = service.handle();
    service.spawn();
    handle
}

/// Rasterize the UMI Light App Tile + Tapered Notch silhouette into ARGB32 (network byte order for SNI).
fn render_umi_tray_icon(size: i32) -> Icon {
    let mut data = vec![0u8; (size * size * 4) as usize];
    let s = size as f64;
    let r = s * 0.25;

    for y in 0..size {
        for x in 0..size {
            let fx = x as f64 + 0.5;
            let fy = y as f64 + 0.5;

            // Rounded-square tile mask
            let dx = (fx - r).min(0.0).abs().max((fx - (s - r)).max(0.0));
            let dy = (fy - r).min(0.0).abs().max((fy - (s - r)).max(0.0));
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > r + 0.5 {
                continue;
            }
            let alpha = ((r + 0.5 - dist).clamp(0.0, 1.0) * 255.0) as u8;

            // Tile light gradient (#F6F9FF -> #DCE6FF)
            let t = ((fx + fy) / (2.0 * s)).clamp(0.0, 1.0);
            let mut pr = (246.0 + (220.0 - 246.0) * t) as u8;
            let mut pg = (249.0 + (230.0 - 249.0) * t) as u8;
            let mut pb = 255u8;

            // Tapered Notch "U" silhouette (#05070E) with Sky-Indigo-Violet top rim
            let nx = (fx / s - 0.5) / 0.38;
            let ny = (fy / s - 0.26) / 0.49;
            let in_top_bar = ny >= 0.0 && ny <= 0.14 && nx.abs() <= 1.0;
            let in_u_body = ny > 0.10
                && ny <= 1.0
                && {
                    let half_w = if ny < 0.55 {
                        0.76
                    } else {
                        let k = ((ny - 0.55) / 0.45).clamp(0.0, 1.0);
                        0.76 * (1.0 - k * k).max(0.0).sqrt()
                    };
                    nx.abs() <= half_w
                };

            if in_top_bar || in_u_body {
                pr = 0x05;
                pg = 0x07;
                pb = 0x0E;

                // Subtle sky-indigo-violet rim highlight on bottom U contour
                if ny > 0.85 {
                    let gx = ((nx + 1.0) * 0.5).clamp(0.0, 1.0);
                    pr = (0x38 as f64 * (1.0 - gx) + 0xA8 as f64 * gx) as u8;
                    pg = (0xBD as f64 * (1.0 - gx) + 0x55 as f64 * gx) as u8;
                    pb = 0xF7;
                }
            }

            // Squircle white eyes + sky glint
            let left_eye = (fx - s * 0.39).abs() <= s * 0.055 && (fy - s * 0.49).abs() <= s * 0.072;
            let right_eye = (fx - s * 0.61).abs() <= s * 0.055 && (fy - s * 0.49).abs() <= s * 0.072;
            if left_eye || right_eye {
                pr = 255;
                pg = 255;
                pb = 255;
            }

            let idx = ((y * size + x) * 4) as usize;
            data[idx] = alpha;
            data[idx + 1] = pr;
            data[idx + 2] = pg;
            data[idx + 3] = pb;
        }
    }

    Icon {
        width: size,
        height: size,
        data,
    }
}
