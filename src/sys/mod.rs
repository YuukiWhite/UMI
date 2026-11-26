pub mod bluez;
pub mod desktop;
pub mod klipper;
pub mod location;
pub mod mpris;
pub mod notifications;
pub mod portals;
pub mod upower;

use crate::ai::{self, AiRequest};
use crate::config::AiConfig;
use crate::quota;
use crate::state::{AppEvent, MediaCommand};
use crate::storage::LocalDb;
use std::sync::{Arc, RwLock};

/// Spawn all KDE Wayland D-Bus, portal, AI, quota, and location background workers onto Tokio.
#[allow(clippy::too_many_arguments)]
pub fn spawn_system_watchers(
    rt: &tokio::runtime::Runtime,
    tx: async_channel::Sender<AppEvent>,
    media_cmd_rx: async_channel::Receiver<MediaCommand>,
    clip_copy_rx: async_channel::Receiver<String>,
    ai_req_rx: async_channel::Receiver<AiRequest>,
    quota_refresh_rx: async_channel::Receiver<()>,
    db: Option<LocalDb>,
    auto_location: bool,
    ai_cfg: Arc<RwLock<AiConfig>>,
) {
    rt.spawn(upower::run_upower_watcher(tx.clone()));
    rt.spawn(bluez::run_bluez_watcher(tx.clone()));
    rt.spawn(notifications::run_notification_monitor(tx.clone()));
    rt.spawn(klipper::run_klipper_watcher(tx.clone(), clip_copy_rx));
    rt.spawn(mpris::run_mpris_watcher(tx.clone(), media_cmd_rx, db.clone()));
    rt.spawn(portals::run_global_shortcuts_portal(tx.clone()));
    rt.spawn(quota::run_quota_watcher(
        tx.clone(),
        quota_refresh_rx,
        db,
        Arc::clone(&ai_cfg),
    ));

    // Ask UMI AI + Smart OCR worker (reads live AiConfig on every request)
    let tx_ai = tx.clone();
    rt.spawn(async move {
        while let Ok(req) = ai_req_rx.recv().await {
            let cfg_snap = ai_cfg.read().map(|c| c.clone()).unwrap_or_default();
            let resp = ai::handle_request(req, &cfg_snap).await;
            let _ = tx_ai.send(AppEvent::AiResponseReady(resp)).await;
        }
    });

    if auto_location {
        rt.spawn(location::detect_and_emit_location(tx));
    }
}
