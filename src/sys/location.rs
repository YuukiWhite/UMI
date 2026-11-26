use crate::state::AppEvent;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use zbus::zvariant::OwnedObjectPath;
use zbus::{Connection, Proxy};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BeaconRequest {
    wifi_access_points: Vec<WifiApPayload>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WifiApPayload {
    mac_address: String,
    signal_strength: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct BeaconResponse {
    location: Option<BeaconLatLng>,
    accuracy: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct BeaconLatLng {
    lat: f64,
    lng: f64,
}

#[derive(Debug, Deserialize)]
struct NominatimResp {
    address: Option<NominatimAddress>,
}

#[derive(Debug, Deserialize)]
struct NominatimAddress {
    city: Option<String>,
    town: Option<String>,
    municipality: Option<String>,
    county: Option<String>,
    state_district: Option<String>,
    state: Option<String>,
}

/// Continuously maintain accurate (~100m-1km) location using nearby Wi-Fi BSSIDs (NetworkManager D-Bus)
/// + BeaconDB (`https://api.beacondb.net/v1/geolocate`) + OSM Nominatim reverse geocoding.
/// Re-checks on startup and every 20 minutes so moving to a new city updates Prayer Times automatically
/// without falling back to inaccurate ISP IP gateways.
pub async fn detect_and_emit_location(tx: async_channel::Sender<AppEvent>) {
    let mut last_coords: Option<(f64, f64)> = None;
    let mut interval = tokio::time::interval(Duration::from_secs(20 * 60));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        interval.tick().await;
        if let Some((lat, lon, acc_m)) = geolocate_via_wifi_beacondb().await {
            // Only accept genuine Wi-Fi triangulation fixes (<= 15 km accuracy), never coarse ISP IP guesses
            if acc_m <= 15_000.0 {
                let moved_significantly = match last_coords {
                    None => true,
                    Some((plat, plon)) => (plat - lat).hypot(plon - lon) > 0.015, // ~1.5 km
                };
                if moved_significantly {
                    let city = reverse_geocode_city(lat, lon)
                        .await
                        .unwrap_or_else(|| format!("{lat:.2}, {lon:.2}"));
                    tracing::info!(
                        "Wi-Fi BeaconDB location fix: {} ({:.4}, {:.4}, ±{:.0}m)",
                        city,
                        lat,
                        lon,
                        acc_m
                    );
                    last_coords = Some((lat, lon));
                    let _ = tx
                        .send(AppEvent::PrayerLocationDetected {
                            city,
                            latitude: lat,
                            longitude: lon,
                        })
                        .await;
                }
            } else {
                tracing::info!(
                    "Ignoring coarse location fix (±{:.0}m); keeping saved city",
                    acc_m
                );
            }
        }
    }
}

async fn geolocate_via_wifi_beacondb() -> Option<(f64, f64, f64)> {
    let aps = scan_networkmanager_wifi_aps().await?;
    if aps.len() < 2 {
        tracing::debug!("Need >= 2 nearby Wi-Fi APs for BeaconDB triangulation (found {})", aps.len());
        return None;
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(7))
        .user_agent("UMI-Dynamic-Island/0.3 (Linux KDE Wayland)")
        .build()
        .ok()?;

    let resp = client
        .post("https://api.beacondb.net/v1/geolocate")
        .json(&BeaconRequest {
            wifi_access_points: aps,
        })
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        tracing::debug!("BeaconDB HTTP {}", resp.status());
        return None;
    }

    let data: BeaconResponse = resp.json().await.ok()?;
    let loc = data.location?;
    let acc = data.accuracy.unwrap_or(500.0);
    Some((loc.lat, loc.lng, acc))
}

async fn scan_networkmanager_wifi_aps() -> Option<Vec<WifiApPayload>> {
    let conn = Connection::system().await.ok()?;
    let nm = Proxy::new(
        &conn,
        "org.freedesktop.NetworkManager",
        "/org/freedesktop/NetworkManager",
        "org.freedesktop.NetworkManager",
    )
    .await
    .ok()?;

    let devices: Vec<OwnedObjectPath> = nm.call("GetDevices", &()).await.ok()?;
    let mut aps = Vec::new();

    for dev_path in devices {
        let Ok(dev) = Proxy::new(
            &conn,
            "org.freedesktop.NetworkManager",
            dev_path.as_str(),
            "org.freedesktop.NetworkManager.Device",
        )
        .await
        else {
            continue;
        };
        let dev_type: u32 = dev.get_property("DeviceType").await.unwrap_or(0);
        // NM_DEVICE_TYPE_WIFI == 2
        if dev_type != 2 {
            continue;
        }

        let Ok(wifi) = Proxy::new(
            &conn,
            "org.freedesktop.NetworkManager",
            dev_path.as_str(),
            "org.freedesktop.NetworkManager.Device.Wireless",
        )
        .await
        else {
            continue;
        };

        let ap_paths: Vec<OwnedObjectPath> = wifi.call("GetAllAccessPoints", &()).await.unwrap_or_default();
        for ap_path in ap_paths {
            let Ok(ap) = Proxy::new(
                &conn,
                "org.freedesktop.NetworkManager",
                ap_path.as_str(),
                "org.freedesktop.NetworkManager.AccessPoint",
            )
            .await
            else {
                continue;
            };

            let ssid_bytes: Vec<u8> = ap.get_property("Ssid").await.unwrap_or_default();
            let ssid = String::from_utf8_lossy(&ssid_bytes);
            // Respect Wi-Fi geolocation opt-out standard (_nomap)
            if ssid.ends_with("_nomap") {
                continue;
            }

            let mac: String = ap.get_property("HwAddress").await.unwrap_or_default();
            if mac.len() < 11 {
                continue;
            }
            let strength: u8 = ap.get_property("Strength").await.unwrap_or(50);
            let freq: u32 = ap.get_property("Frequency").await.unwrap_or(0);
            // Convert NM 0..100 strength to approximate dBm (-100..-35)
            let dbm = -100 + ((strength as i32 * 65) / 100);

            aps.push(WifiApPayload {
                mac_address: mac.to_ascii_lowercase(),
                signal_strength: dbm,
                frequency: (freq > 0).then_some(freq),
            });
        }
    }

    // Keep strongest 15 unique BSSIDs
    aps.sort_by(|a, b| b.signal_strength.cmp(&a.signal_strength));
    aps.dedup_by(|a, b| a.mac_address == b.mac_address);
    aps.truncate(15);
    Some(aps)
}

async fn reverse_geocode_city(lat: f64, lon: f64) -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(6))
        .user_agent("UMI-Dynamic-Island/0.3 (https://github.com/umi-notch)")
        .build()
        .ok()?;

    let resp = client
        .get("https://nominatim.openstreetmap.org/reverse")
        .query(&[
            ("format", "jsonv2"),
            ("lat", &format!("{lat:.5}")),
            ("lon", &format!("{lon:.5}")),
            ("zoom", "10"),
        ])
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let data: NominatimResp = resp.json().await.ok()?;
    let addr = data.address?;
    addr.city
        .or(addr.town)
        .or(addr.municipality)
        .or(addr.county)
        .or(addr.state_district)
        .or(addr.state)
        .map(|s| s.replace("Kota ", "").replace("Kabupaten ", "").trim().to_string())
}
