use crate::state::AppEvent;
use futures_util::StreamExt;
use std::time::Duration;
use zbus::{Connection, Proxy};

/// Watch both `org.freedesktop.UPower` (`OnBattery`) and `DisplayDevice` (`State`, `Percentage`)
/// via PropertiesChanged streams for instant (<20ms) AC plug-in Peek alerts and live battery %.
pub async fn run_upower_watcher(tx: async_channel::Sender<AppEvent>) {
    let conn = match Connection::system().await {
        Ok(c) => c,
        Err(err) => {
            tracing::warn!("UPower watcher skipped (system bus unavailable): {err}");
            return;
        }
    };

    let upower_proxy = Proxy::new(
        &conn,
        "org.freedesktop.UPower",
        "/org/freedesktop/UPower",
        "org.freedesktop.UPower",
    )
    .await
    .ok();

    let dev_proxy = match Proxy::new(
        &conn,
        "org.freedesktop.UPower",
        "/org/freedesktop/UPower/devices/DisplayDevice",
        "org.freedesktop.UPower.Device",
    )
    .await
    {
        Ok(p) => p,
        Err(err) => {
            tracing::warn!("UPower DisplayDevice proxy failed: {err}");
            return;
        }
    };

    let mut prev_plugged: Option<bool> = None;
    read_and_emit(upower_proxy.as_ref(), &dev_proxy, &mut prev_plugged, &tx).await;

    let mut on_bat_stream = match upower_proxy.as_ref() {
        Some(p) => Some(p.receive_property_changed::<bool>("OnBattery").await),
        None => None,
    };
    let mut state_stream = dev_proxy.receive_property_changed::<u32>("State").await;
    let mut pct_stream = dev_proxy.receive_property_changed::<f64>("Percentage").await;
    let mut interval = tokio::time::interval(Duration::from_secs(20));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                read_and_emit(upower_proxy.as_ref(), &dev_proxy, &mut prev_plugged, &tx).await;
            }
            Some(_) = async {
                match on_bat_stream.as_mut() {
                    Some(s) => s.next().await,
                    None => std::future::pending().await,
                }
            } => {
                read_and_emit(upower_proxy.as_ref(), &dev_proxy, &mut prev_plugged, &tx).await;
            }
            Some(_) = state_stream.next() => {
                read_and_emit(upower_proxy.as_ref(), &dev_proxy, &mut prev_plugged, &tx).await;
            }
            Some(_) = pct_stream.next() => {
                read_and_emit(upower_proxy.as_ref(), &dev_proxy, &mut prev_plugged, &tx).await;
            }
        }
    }
}

async fn read_and_emit(
    upower_proxy: Option<&Proxy<'_>>,
    dev_proxy: &Proxy<'_>,
    prev_plugged: &mut Option<bool>,
    tx: &async_channel::Sender<AppEvent>,
) {
    let pct_f: f64 = dev_proxy.get_property("Percentage").await.unwrap_or(0.0);
    let state: u32 = dev_proxy.get_property("State").await.unwrap_or(0);
    // UPower DeviceState: 1 = Charging, 4 = FullyCharged, 5 = PendingCharge
    let dev_charging = matches!(state, 1 | 4 | 5);
    let on_ac = match upower_proxy {
        Some(p) => p
            .get_property::<bool>("OnBattery")
            .await
            .map(|on_bat| !on_bat)
            .unwrap_or(dev_charging),
        None => dev_charging,
    };
    let plugged = on_ac || dev_charging;
    let pct = pct_f.round().clamp(0.0, 100.0) as u8;
    let just_plugged_in = matches!(*prev_plugged, Some(false)) && plugged;
    *prev_plugged = Some(plugged);

    if pct > 0 || plugged {
        let _ = tx
            .send(AppEvent::BatteryUpdated {
                pct,
                charging: plugged,
                just_plugged_in,
            })
            .await;
    }
}
