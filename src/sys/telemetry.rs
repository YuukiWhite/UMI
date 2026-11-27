use crate::config::WidgetKind;
use serde::Deserialize;
use std::fs;
use std::time::Instant;

#[derive(Debug, Clone, Default)]
pub struct WeatherSnapshot {
    pub city: String,
    pub temp_c: f64,
    pub high_c: f64,
    pub low_c: f64,
    pub humidity_pct: u8,
    pub weather_code: u16,
    pub ready: bool,
}

impl WeatherSnapshot {
    pub fn description_id(&self) -> &'static str {
        if !self.ready {
            return "Memuat cuaca...";
        }
        match self.weather_code {
            0 => "Cerah",
            1 | 2 => "Cerah Berawan",
            3 => "Berawan",
            45 | 48 => "Berkabut",
            51..=57 => "Gerimis",
            61..=67 | 80..=82 => "Hujan",
            71..=77 | 85 | 86 => "Salju",
            95..=99 => "Hujan Petir",
            _ => "Berawan",
        }
    }
}

#[derive(Debug, Clone)]
pub struct WidgetTelemetry {
    pub cpu_pct: f64,
    pub ram_used_gb: f64,
    pub ram_total_gb: f64,
    pub ram_pct: f64,
    pub load_avg: f64,
    pub cpu_temp_c: Option<f64>,
    pub nvme_temp_c: Option<f64>,
    pub disk_used_gb: f64,
    pub disk_total_gb: f64,
    pub disk_pct: f64,
    pub net_down_bps: f64,
    pub net_up_bps: f64,
    pub net_iface: String,
    pub weather: WeatherSnapshot,
    pub quick_note: String,
    // Internal delta trackers
    prev_cpu_idle: u64,
    prev_cpu_total: u64,
    prev_rx_bytes: u64,
    prev_tx_bytes: u64,
    prev_net_time: Option<Instant>,
    last_weather_fetch: Option<Instant>,
}

impl Default for WidgetTelemetry {
    fn default() -> Self {
        Self {
            cpu_pct: 0.0,
            ram_used_gb: 0.0,
            ram_total_gb: 0.0,
            ram_pct: 0.0,
            load_avg: 0.0,
            cpu_temp_c: None,
            nvme_temp_c: None,
            disk_used_gb: 0.0,
            disk_total_gb: 0.0,
            disk_pct: 0.0,
            net_down_bps: 0.0,
            net_up_bps: 0.0,
            net_iface: "Online".to_string(),
            weather: WeatherSnapshot::default(),
            quick_note: "Klik untuk menulis catatan cepat...".to_string(),
            prev_cpu_idle: 0,
            prev_cpu_total: 0,
            prev_rx_bytes: 0,
            prev_tx_bytes: 0,
            prev_net_time: None,
            last_weather_fetch: None,
        }
    }
}

impl WidgetTelemetry {
    /// Zero-overhead refresh: ONLY reads Linux `/proc` / `/sys` files required by active slots.
    pub fn refresh_for_slots(&mut self, left: WidgetKind, right: WidgetKind) {
        let needs = |k: WidgetKind| left == k || right == k;

        if needs(WidgetKind::SystemMonitor) {
            self.read_cpu();
            self.read_ram();
            self.read_loadavg();
            self.read_temps();
        }
        if needs(WidgetKind::DiskThermal) {
            self.read_disk_root();
            if !needs(WidgetKind::SystemMonitor) {
                self.read_temps();
            }
        }
        if needs(WidgetKind::NetworkSpeed) {
            self.read_network();
        }
    }

    pub fn should_fetch_weather(&mut self, left: WidgetKind, right: WidgetKind) -> bool {
        if left != WidgetKind::Weather && right != WidgetKind::Weather {
            return false;
        }
        let stale = match self.last_weather_fetch {
            None => true,
            Some(ts) => ts.elapsed().as_secs() >= 30 * 60,
        };
        if stale {
            self.last_weather_fetch = Some(Instant::now());
            true
        } else {
            false
        }
    }

    fn read_cpu(&mut self) {
        let Ok(raw) = fs::read_to_string("/proc/stat") else {
            return;
        };
        let Some(first) = raw.lines().next() else {
            return;
        };
        let vals: Vec<u64> = first
            .split_whitespace()
            .skip(1)
            .filter_map(|s| s.parse::<u64>().ok())
            .collect();
        if vals.len() < 4 {
            return;
        }
        let idle = vals[3].saturating_add(*vals.get(4).unwrap_or(&0));
        let total: u64 = vals.iter().sum();
        if self.prev_cpu_total > 0 && total > self.prev_cpu_total {
            let dt = (total - self.prev_cpu_total) as f64;
            let di = idle.saturating_sub(self.prev_cpu_idle) as f64;
            self.cpu_pct = (((dt - di) / dt) * 100.0).clamp(0.0, 100.0);
        }
        self.prev_cpu_idle = idle;
        self.prev_cpu_total = total;
    }

    fn read_ram(&mut self) {
        let Ok(raw) = fs::read_to_string("/proc/meminfo") else {
            return;
        };
        let mut total_kb = 0u64;
        let mut avail_kb = 0u64;
        for line in raw.lines() {
            if let Some(rest) = line.strip_prefix("MemTotal:") {
                total_kb = rest
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
            } else if let Some(rest) = line.strip_prefix("MemAvailable:") {
                avail_kb = rest
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
            }
        }
        if total_kb > 0 {
            let used_kb = total_kb.saturating_sub(avail_kb);
            self.ram_total_gb = total_kb as f64 / (1024.0 * 1024.0);
            self.ram_used_gb = used_kb as f64 / (1024.0 * 1024.0);
            self.ram_pct = ((used_kb as f64 / total_kb as f64) * 100.0).clamp(0.0, 100.0);
        }
    }

    fn read_loadavg(&mut self) {
        if let Ok(raw) = fs::read_to_string("/proc/loadavg") {
            if let Some(first) = raw.split_whitespace().next() {
                if let Ok(v) = first.parse::<f64>() {
                    self.load_avg = v;
                }
            }
        }
    }

    fn read_temps(&mut self) {
        let mut cpu_t: Option<f64> = None;
        let mut nvme_t: Option<f64> = None;
        if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
            for entry in entries.flatten() {
                let p = entry.path();
                let name = fs::read_to_string(p.join("name"))
                    .unwrap_or_default()
                    .trim()
                    .to_ascii_lowercase();
                let temp_raw = fs::read_to_string(p.join("temp1_input"))
                    .ok()
                    .and_then(|s| s.trim().parse::<f64>().ok())
                    .map(|milli| milli / 1000.0);
                let Some(t) = temp_raw else { continue };
                if !(5.0..=125.0).contains(&t) {
                    continue;
                }
                if (name.contains("k10temp") || name.contains("coretemp") || name.contains("zenpower") || name.contains("cpu"))
                    && cpu_t.is_none()
                {
                    cpu_t = Some(t);
                } else if name.contains("nvme") && nvme_t.is_none() {
                    nvme_t = Some(t);
                }
            }
        }
        if cpu_t.is_none() {
            if let Ok(entries) = fs::read_dir("/sys/class/thermal") {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if let Ok(s) = fs::read_to_string(p.join("temp")) {
                        if let Ok(milli) = s.trim().parse::<f64>() {
                            let t = milli / 1000.0;
                            if (10.0..=120.0).contains(&t) {
                                cpu_t = Some(t);
                                break;
                            }
                        }
                    }
                }
            }
        }
        self.cpu_temp_c = cpu_t;
        self.nvme_temp_c = nvme_t;
    }

    fn read_disk_root(&mut self) {
        #[repr(C)]
        struct StatVfs {
            f_bsize: u64,
            f_frsize: u64,
            f_blocks: u64,
            f_bfree: u64,
            f_bavail: u64,
            f_files: u64,
            f_ffree: u64,
            f_favail: u64,
            f_fsid: u64,
            f_flag: u64,
            f_namemax: u64,
            __f_spare: [i32; 6],
        }
        extern "C" {
            fn statvfs(path: *const i8, buf: *mut StatVfs) -> i32;
        }
        let mut st = std::mem::MaybeUninit::<StatVfs>::zeroed();
        let rc = unsafe { statvfs(b"/\0".as_ptr() as *const i8, st.as_mut_ptr()) };
        if rc == 0 {
            let st = unsafe { st.assume_init() };
            let block = if st.f_frsize > 0 { st.f_frsize } else { st.f_bsize };
            let total = st.f_blocks.saturating_mul(block) as f64;
            let free = st.f_bavail.saturating_mul(block) as f64;
            let used = (total - free).max(0.0);
            if total > 0.0 {
                let gb = 1024.0 * 1024.0 * 1024.0;
                self.disk_total_gb = total / gb;
                self.disk_used_gb = used / gb;
                self.disk_pct = ((used / total) * 100.0).clamp(0.0, 100.0);
            }
        }
    }

    fn read_network(&mut self) {
        let Ok(raw) = fs::read_to_string("/proc/net/dev") else {
            return;
        };
        let mut rx_total = 0u64;
        let mut tx_total = 0u64;
        let mut active_iface = String::new();
        let mut max_rx = 0u64;
        for line in raw.lines().skip(2) {
            let Some((iface_raw, rest)) = line.split_once(':') else {
                continue;
            };
            let iface = iface_raw.trim();
            if iface == "lo" || iface.starts_with("docker") || iface.starts_with("veth") || iface.starts_with("virbr") {
                continue;
            }
            let cols: Vec<u64> = rest
                .split_whitespace()
                .filter_map(|s| s.parse::<u64>().ok())
                .collect();
            if cols.len() >= 9 {
                rx_total = rx_total.saturating_add(cols[0]);
                tx_total = tx_total.saturating_add(cols[8]);
                if cols[0] > max_rx {
                    max_rx = cols[0];
                    active_iface = iface.to_string();
                }
            }
        }
        let now = Instant::now();
        if let Some(prev_t) = self.prev_net_time {
            let dt = now.duration_since(prev_t).as_secs_f64().max(0.2);
            self.net_down_bps = rx_total.saturating_sub(self.prev_rx_bytes) as f64 / dt;
            self.net_up_bps = tx_total.saturating_sub(self.prev_tx_bytes) as f64 / dt;
        }
        self.prev_rx_bytes = rx_total;
        self.prev_tx_bytes = tx_total;
        self.prev_net_time = Some(now);
        if !active_iface.is_empty() {
            self.net_iface = if active_iface.starts_with("wl") {
                format!("Wi-Fi · {active_iface}")
            } else {
                format!("LAN · {active_iface}")
            };
        }
    }
}

pub fn format_speed(bps: f64) -> String {
    if bps >= 1024.0 * 1024.0 {
        format!("{:.1} MB/s", bps / (1024.0 * 1024.0))
    } else if bps >= 1024.0 {
        format!("{:.0} KB/s", bps / 1024.0)
    } else {
        format!("{:.0} B/s", bps.max(0.0))
    }
}

#[derive(Deserialize)]
struct OpenMeteoResp {
    current: Option<OpenMeteoCurrent>,
    daily: Option<OpenMeteoDaily>,
}

#[derive(Deserialize)]
struct OpenMeteoCurrent {
    temperature_2m: f64,
    relative_humidity_2m: f64,
    weather_code: u16,
}

#[derive(Deserialize)]
struct OpenMeteoDaily {
    temperature_2m_max: Vec<f64>,
    temperature_2m_min: Vec<f64>,
}

/// Fetch live local weather via Open-Meteo (free, zero API key, 30-min cached).
pub async fn fetch_open_meteo(city: String, lat: f64, lon: f64) -> Option<WeatherSnapshot> {
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={lat:.4}&longitude={lon:.4}&current=temperature_2m,relative_humidity_2m,weather_code&daily=temperature_2m_max,temperature_2m_min&timezone=auto&forecast_days=1"
    );
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(6))
        .build()
        .ok()?;
    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body: OpenMeteoResp = resp.json().await.ok()?;
    let cur = body.current?;
    let hi = body
        .daily
        .as_ref()
        .and_then(|d| d.temperature_2m_max.first().copied())
        .unwrap_or(cur.temperature_2m + 2.0);
    let lo = body
        .daily
        .as_ref()
        .and_then(|d| d.temperature_2m_min.first().copied())
        .unwrap_or(cur.temperature_2m - 4.0);
    Some(WeatherSnapshot {
        city,
        temp_c: cur.temperature_2m,
        high_c: hi,
        low_c: lo,
        humidity_pct: cur.relative_humidity_2m.round().clamp(0.0, 100.0) as u8,
        weather_code: cur.weather_code,
        ready: true,
    })
}
