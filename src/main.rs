mod ai;
mod cli;
mod config;
mod domain;
mod ipc;
mod lyrics;
mod quota;
mod state;
mod storage;
mod sys;
mod ui;

use ai::AiRequest;
use anyhow::Result;
use clap::Parser;
use cli::{Cli, CliCommand};
use gtk4::gio;
use gtk4::prelude::*;
use gtk4::Application;
use state::{AppEvent, MediaCommand};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};
use tracing_subscriber::EnvFilter;
use ui::notch::window::build_notch_window;
use ui::tray::{spawn_tray, TrayFlags};

fn main() -> Result<()> {
    // Use lightweight 2D Cairo Wayland SHM renderer by default (avoids ~65MB Mesa Vulkan/GL RSS overhead)
    // and limit glibc malloc arenas for minimal memory footprint (~25-40 MB RSS).
    if std::env::var_os("GSK_RENDERER").is_none() {
        std::env::set_var("GSK_RENDERER", "cairo");
    }
    if std::env::var_os("MALLOC_ARENA_MAX").is_none() {
        std::env::set_var("MALLOC_ARENA_MAX", "2");
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("umi=info")),
        )
        .init();

    let args = Cli::parse();

    // 1 Tokio worker thread + 2 blocking slots is plenty for D-Bus/HTTP watchers and avoids 16+ idle thread stacks
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .max_blocking_threads(2)
        .enable_all()
        .build()?;

    // Handle CLI commands directly (SetKey -> KDE Wallet, others -> D-Bus IPC to running daemon)
    if let Some(cmd) = args.command {
        if let CliCommand::SetKey { provider, api_key } = cmd {
            rt.block_on(storage::secrets::save_secret(&provider, &api_key))?;
            println!(
                "Saved encrypted {} key to KDE Wallet (folder: umi).",
                storage::secrets::key_name_for_provider(&provider)
            );
            return Ok(());
        }
        return rt.block_on(ipc::send_cli_command(cmd));
    }

    let cfg = config::load_or_create()?;
    if let Err(err) = sys::desktop::install_desktop_entry() {
        tracing::warn!("Desktop entry install skipped ({err})");
    }
    if let Err(err) = sys::desktop::sync_autostart(cfg.general.start_at_login) {
        tracing::warn!("Autostart sync skipped ({err})");
    }
    let db = match storage::LocalDb::open_default() {
        Ok(db) => {
            tracing::info!("SQLite database ready");
            Some(db)
        }
        Err(err) => {
            tracing::warn!("SQLite init failed ({err}); continuing without cache");
            None
        }
    };

    let (tx, rx) = async_channel::unbounded::<AppEvent>();
    let (media_cmd_tx, media_cmd_rx) = async_channel::unbounded::<MediaCommand>();
    let (clip_copy_tx, clip_copy_rx) = async_channel::unbounded::<String>();
    let (ai_req_tx, ai_req_rx) = async_channel::unbounded::<AiRequest>();
    let (quota_refresh_tx, quota_refresh_rx) = async_channel::unbounded::<()>();
    let status_text = Arc::new(RwLock::new("starting".to_string()));

    let ipc_tx = tx.clone();
    let ipc_status = Arc::clone(&status_text);
    let ipc_guard = rt.block_on(async move {
        match ipc::start_server(ipc_tx, ipc_status).await {
            Ok(conn) => {
                tracing::info!("D-Bus IPC listening on {}", ipc::BUS_NAME);
                Ok(conn)
            }
            Err(err) => Err(err),
        }
    });
    let _ipc_guard = match ipc_guard {
        Ok(conn) => conn,
        Err(err) => {
            tracing::info!("Existing UMI instance detected ({err}); toggling it");
            let _ = rt.block_on(ipc::send_cli_command(CliCommand::Toggle));
            return Ok(());
        }
    };

    let ai_cfg = Arc::new(RwLock::new(cfg.ai.clone()));
    sys::spawn_system_watchers(
        &rt,
        tx.clone(),
        media_cmd_rx,
        clip_copy_rx,
        ai_req_rx,
        quota_refresh_rx,
        db.clone(),
        cfg.prayer.auto_location,
        Arc::clone(&ai_cfg),
    );

    let tray_flags = TrayFlags {
        start_at_login: Arc::new(AtomicBool::new(cfg.general.start_at_login)),
        mascot_eyes: Arc::new(AtomicBool::new(cfg.general.idle_mascot_eyes)),
        minimal_mode: Arc::new(AtomicBool::new(cfg.general.minimal_notch_mode)),
    };
    let _tray_handle = spawn_tray(tx.clone(), tray_flags.clone());

    let app = Application::builder()
        .application_id("org.umi.Notch")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();

    let rx_cell = std::cell::RefCell::new(Some(rx));
    let cfg_cell = std::cell::RefCell::new(Some(cfg));
    let db_cell = std::cell::RefCell::new(db);
    let rt_handle = rt.handle().clone();
    let event_tx = tx.clone();
    app.connect_activate(move |app| {
        if let Some(win) = app.active_window() {
            win.present();
            return;
        }
        let Some(event_rx) = rx_cell.borrow_mut().take() else {
            return;
        };
        let Some(cfg) = cfg_cell.borrow_mut().take() else {
            return;
        };
        let window = build_notch_window(
            app,
            cfg,
            db_cell.borrow_mut().take(),
            event_rx,
            event_tx.clone(),
            media_cmd_tx.clone(),
            clip_copy_tx.clone(),
            ai_req_tx.clone(),
            quota_refresh_tx.clone(),
            Arc::clone(&ai_cfg),
            rt_handle.clone(),
            Arc::clone(&status_text),
            tray_flags.clone(),
        );
        window.present();
    });

    let exit_code = app.run_with_args::<&str>(&[]);
    tracing::info!("UMI exited with code {:?}", exit_code);
    Ok(())
}
