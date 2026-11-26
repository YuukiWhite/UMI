use crate::state::AppEvent;
use futures_util::StreamExt;
use std::collections::HashMap;
use zbus::zvariant::OwnedValue;
use zbus::{Connection, MatchRule, MessageStream, Proxy};

/// Watch BlueZ Device1 PropertiesChanged signals on the system bus for Bluetooth audio connections.
pub async fn run_bluez_watcher(tx: async_channel::Sender<AppEvent>) {
    let conn = match Connection::system().await {
        Ok(c) => c,
        Err(err) => {
            tracing::warn!("BlueZ watcher skipped (system bus unavailable): {err}");
            return;
        }
    };

    let rule = match MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .sender("org.bluez")
        .and_then(|b| b.interface("org.freedesktop.DBus.Properties"))
        .and_then(|b| b.member("PropertiesChanged"))
    {
        Ok(b) => b.build(),
        Err(err) => {
            tracing::warn!("BlueZ match rule failed: {err}");
            return;
        }
    };

    let mut stream = match MessageStream::for_match_rule(rule, &conn, Some(64)).await {
        Ok(s) => s,
        Err(err) => {
            tracing::warn!("BlueZ signal subscription failed: {err}");
            return;
        }
    };

    while let Some(Ok(msg)) = stream.next().await {
        let Some(path) = msg.header().path().map(|p| p.to_string()) else {
            continue;
        };
        let Ok((iface, changed, _invalidated)) =
            msg.body()
                .deserialize::<(String, HashMap<String, OwnedValue>, Vec<String>)>()
        else {
            continue;
        };
        if iface != "org.bluez.Device1" {
            continue;
        }
        let connected = changed
            .get("Connected")
            .and_then(|v| bool::try_from(v.clone()).ok());
        if connected != Some(true) {
            continue;
        }

        // Read device metadata & optional Battery1 percentage
        let Ok(dev_proxy) = Proxy::new(&conn, "org.bluez", path.as_str(), "org.bluez.Device1").await
        else {
            continue;
        };

        let icon: String = dev_proxy.get_property("Icon").await.unwrap_or_default();
        let uuids: Vec<String> = dev_proxy.get_property("UUIDs").await.unwrap_or_default();
        let dev_class: u32 = dev_proxy.get_property("Class").await.unwrap_or(0);
        // Major Device Class 0x0400 = Audio/Video (headphones, earbuds, speakers)
        let is_audio_class = (dev_class & 0x1F00) == 0x0400;
        let is_audio = is_audio_class
            || icon.contains("audio")
            || icon.contains("head")
            || uuids.iter().any(|u| {
                let low = u.to_ascii_lowercase();
                low.starts_with("0000110b")
                    || low.starts_with("0000111e")
                    || low.starts_with("00001108")
                    || low.starts_with("0000110d")
            });
        if !is_audio {
            continue;
        }

        let name: String = dev_proxy
            .get_property("Alias")
            .await
            .or_else(|_| Err(()))
            .ok()
            .filter(|s: &String| !s.is_empty())
            .unwrap_or_else(|| "Bluetooth Audio".to_string());

        let battery_pct = match Proxy::new(&conn, "org.bluez", path.as_str(), "org.bluez.Battery1").await
        {
            Ok(bp) => bp.get_property::<u8>("Percentage").await.unwrap_or(100),
            Err(_) => 100,
        };

        let _ = tx
            .send(AppEvent::BluetoothConnected {
                device: name,
                battery_pct,
            })
            .await;
    }
}
