use crate::cli::CliCommand;
use crate::config::NotchPageId;
use crate::state::AppEvent;
use anyhow::Result;
use std::sync::{Arc, RwLock};
use zbus::{interface, proxy, Connection};

pub const BUS_NAME: &str = "org.umi.Notch";
pub const OBJECT_PATH: &str = "/org/umi/Notch";

pub struct UmiIpcService {
    tx: async_channel::Sender<AppEvent>,
    status_text: Arc<RwLock<String>>,
}

impl UmiIpcService {
    pub fn new(tx: async_channel::Sender<AppEvent>, status_text: Arc<RwLock<String>>) -> Self {
        Self { tx, status_text }
    }
}

#[interface(name = "org.umi.Notch")]
impl UmiIpcService {
    async fn toggle(&self) -> zbus::fdo::Result<()> {
        let _ = self.tx.send(AppEvent::ToggleNotch).await;
        Ok(())
    }

    async fn ask(&self) -> zbus::fdo::Result<()> {
        let _ = self.tx.send(AppEvent::OpenAskAi).await;
        Ok(())
    }

    async fn ocr(&self) -> zbus::fdo::Result<()> {
        let _ = self.tx.send(AppEvent::TriggerOcr).await;
        Ok(())
    }

    async fn open_settings(&self) -> zbus::fdo::Result<()> {
        let _ = self.tx.send(AppEvent::OpenSettings).await;
        Ok(())
    }

    async fn jump_page(&self, name: &str) -> zbus::fdo::Result<bool> {
        if let Some(page) = NotchPageId::from_cli_name(name) {
            let _ = self.tx.send(AppEvent::JumpToPage(page)).await;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn status(&self) -> zbus::fdo::Result<String> {
        Ok(self
            .status_text
            .read()
            .map(|s| s.clone())
            .unwrap_or_else(|_| "unknown".to_string()))
    }
}

#[proxy(
    interface = "org.umi.Notch",
    default_service = "org.umi.Notch",
    default_path = "/org/umi/Notch"
)]
trait UmiNotch {
    async fn toggle(&self) -> zbus::Result<()>;
    async fn ask(&self) -> zbus::Result<()>;
    async fn ocr(&self) -> zbus::Result<()>;
    async fn open_settings(&self) -> zbus::Result<()>;
    async fn jump_page(&self, name: &str) -> zbus::Result<bool>;
    async fn status(&self) -> zbus::Result<String>;
}

/// Start the D-Bus single-instance service on the session bus.
pub async fn start_server(
    tx: async_channel::Sender<AppEvent>,
    status_text: Arc<RwLock<String>>,
) -> Result<Connection> {
    let service = UmiIpcService::new(tx, status_text);
    let conn = zbus::connection::Builder::session()?
        .name(BUS_NAME)?
        .serve_at(OBJECT_PATH, service)?
        .build()
        .await?;
    Ok(conn)
}

/// Forward a CLI subcommand to an already-running UMI instance over D-Bus.
pub async fn send_cli_command(cmd: CliCommand) -> Result<()> {
    let conn = Connection::session().await?;
    let proxy = UmiNotchProxy::new(&conn).await?;
    match cmd {
        CliCommand::Toggle => proxy.toggle().await?,
        CliCommand::Ask => proxy.ask().await?,
        CliCommand::Ocr => proxy.ocr().await?,
        CliCommand::Settings => proxy.open_settings().await?,
        CliCommand::Page { name } => {
            let ok = proxy.jump_page(&name).await?;
            if !ok {
                anyhow::bail!(
                    "Unknown page '{name}'. Try: music, focus, prayer, water, shelf, todo, widgets, ai, limits"
                );
            }
        }
        CliCommand::Status => {
            let state = proxy.status().await?;
            println!("{state}");
        }
        CliCommand::SetKey { .. } => {}
    }
    Ok(())
}
