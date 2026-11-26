use crate::config::PrayerConfig;
use chrono::{Datelike, Local, Timelike};

#[derive(Debug, Clone)]
pub struct PrayerEntry {
    pub name: &'static str,
    pub minutes_from_midnight: i32,
}

impl PrayerEntry {
    pub fn hh_mm(&self) -> String {
        let m = self.minutes_from_midnight.rem_euclid(24 * 60);
        format!("{:02}:{:02}", m / 60, m % 60)
    }
}

#[derive(Debug, Clone)]
pub struct PrayerSchedule {
    pub city: String,
    pub entries: [PrayerEntry; 5],
    pub next_idx: usize,
    pub countdown_text: String,
    pub segment_progress: f64,
    pub sun_arc_progress: f64,
}

/// 100% offline astronomical prayer times calculator (Kemenag RI 20° / 18°, MWL, ISNA).
pub fn compute_today_schedule(cfg: &PrayerConfig) -> PrayerSchedule {
    let now = Local::now();
    let tz_hours = now.offset().local_minus_utc() as f64 / 3600.0;
    let now_min = (now.hour() * 60 + now.minute()) as i32;
    compute_schedule_for_date(
        &cfg.city,
        cfg.latitude,
        cfg.longitude,
        &cfg.method,
        now.year(),
        now.month(),
        now.day(),
        tz_hours,
        now_min,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn compute_schedule_for_date(
    city: &str,
    lat: f64,
    lon: f64,
    method: &str,
    year: i32,
    month: u32,
    day: u32,
    tz_hours: f64,
    now_min: i32,
) -> PrayerSchedule {
    let (fajr_angle, isha_angle, ihtiyati_min) = match method.to_ascii_lowercase().as_str() {
        "mwl" => (18.0, 17.0, 0.0),
        "isna" => (15.0, 15.0, 0.0),
        _ => (20.0, 18.0, 2.0), // Kemenag RI standard
    };

    let jd = julian_day(year, month as i32, day as i32);
    let d = jd - 2451545.0;

    // Mean solar coordinates (Jean Meeus / USNO approximation, < 1 min error)
    let g = (357.529 + 0.98560028 * d).to_radians();
    let q = (280.459 + 0.98564736 * d).rem_euclid(360.0);
    let l = (q + 1.915 * g.sin() + 0.020 * (2.0 * g).sin()).to_radians();
    let e = (23.439 - 0.00000036 * d).to_radians();

    let ra = (e.cos() * l.sin()).atan2(l.cos()).to_degrees().rem_euclid(360.0) / 15.0;
    let decl = (e.sin() * l.sin()).asin();
    let mut eqt = q / 15.0 - ra;
    if eqt > 12.0 {
        eqt -= 24.0;
    } else if eqt < -12.0 {
        eqt += 24.0;
    }

    let dhuhr_h = 12.0 + tz_hours - (lon / 15.0) - eqt;
    let lat_r = lat.to_radians();

    let fajr_h = dhuhr_h - hour_angle(-fajr_angle, lat_r, decl);
    let asr_alt = (1.0 / (1.0 + (lat_r - decl).abs().tan())).atan().to_degrees();
    let asr_h = dhuhr_h + hour_angle(asr_alt, lat_r, decl);
    let maghrib_h = dhuhr_h + hour_angle(-0.8333, lat_r, decl);
    let isha_h = dhuhr_h + hour_angle(-isha_angle, lat_r, decl);

    let to_min = |h: f64, add: f64| -> i32 { (h * 60.0 + add).round() as i32 };
    let entries = [
        PrayerEntry { name: "Subuh", minutes_from_midnight: to_min(fajr_h, ihtiyati_min) },
        PrayerEntry { name: "Dzuhur", minutes_from_midnight: to_min(dhuhr_h, ihtiyati_min) },
        PrayerEntry { name: "Ashar", minutes_from_midnight: to_min(asr_h, ihtiyati_min) },
        PrayerEntry { name: "Maghrib", minutes_from_midnight: to_min(maghrib_h, ihtiyati_min) },
        PrayerEntry { name: "Isya", minutes_from_midnight: to_min(isha_h, ihtiyati_min) },
    ];

    let next_idx = entries
        .iter()
        .position(|e| e.minutes_from_midnight > now_min)
        .unwrap_or(0);

    let next_m = entries[next_idx].minutes_from_midnight;
    let prev_m = if next_idx == 0 {
        entries[4].minutes_from_midnight - 24 * 60
    } else {
        entries[next_idx - 1].minutes_from_midnight
    };
    let target_m = if next_idx == 0 && now_min >= entries[4].minutes_from_midnight {
        next_m + 24 * 60
    } else {
        next_m
    };

    let diff = (target_m - now_min).max(0);
    let countdown_text = if diff >= 60 {
        format!("in {} h {} min", diff / 60, diff % 60)
    } else {
        format!("in {diff} min")
    };

    let span = (target_m - prev_m).max(1) as f64;
    let segment_progress = ((now_min - prev_m) as f64 / span).clamp(0.0, 1.0);

    let subuh_m = entries[0].minutes_from_midnight as f64;
    let isya_m = entries[4].minutes_from_midnight as f64;
    let sun_arc_progress = ((now_min as f64 - subuh_m) / (isya_m - subuh_m).max(1.0)).clamp(0.0, 1.0);

    PrayerSchedule {
        city: city.to_string(),
        entries,
        next_idx,
        countdown_text,
        segment_progress,
        sun_arc_progress,
    }
}

fn hour_angle(altitude_deg: f64, lat_rad: f64, decl_rad: f64) -> f64 {
    let num = altitude_deg.to_radians().sin() - lat_rad.sin() * decl_rad.sin();
    let den = lat_rad.cos() * decl_rad.cos();
    let cos_t = (num / den).clamp(-1.0, 1.0);
    cos_t.acos().to_degrees() / 15.0
}

fn julian_day(year: i32, month: i32, day: i32) -> f64 {
    let (y, m) = if month <= 2 {
        (year - 1, month + 12)
    } else {
        (year, month)
    };
    let a = (y as f64 / 100.0).floor();
    let b = 2.0 - a + (a / 4.0).floor();
    (365.25 * (y + 4716) as f64).floor()
        + (30.6001 * (m + 1) as f64).floor()
        + day as f64
        + b
        - 1524.5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bandung_kemenag_schedule_within_expected_bounds() {
        let sched = compute_schedule_for_date("Bandung", -6.9175, 107.6191, "kemenag", 2026, 11, 26, 7.0, 14 * 60 + 30);
        // Subuh ~04:00-04:20, Dzuhur ~11:35-11:55, Ashar ~14:55-15:20, Maghrib ~17:45-18:05, Isya ~19:00-19:20
        assert!((235..=275).contains(&sched.entries[0].minutes_from_midnight), "Subuh {}", sched.entries[0].hh_mm());
        assert!((690..=725).contains(&sched.entries[1].minutes_from_midnight), "Dzuhur {}", sched.entries[1].hh_mm());
        assert!((885..=935).contains(&sched.entries[2].minutes_from_midnight), "Ashar {}", sched.entries[2].hh_mm());
        assert!((1060..=1100).contains(&sched.entries[3].minutes_from_midnight), "Maghrib {}", sched.entries[3].hh_mm());
        assert!((1130..=1175).contains(&sched.entries[4].minutes_from_midnight), "Isya {}", sched.entries[4].hh_mm());
        assert_eq!(sched.entries[sched.next_idx].name, "Ashar");
    }
}
