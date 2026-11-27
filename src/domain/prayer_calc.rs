use crate::config::{FaithTradition, PrayerConfig};
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
    pub faith: FaithTradition,
    pub city: String,
    #[allow(dead_code)]
    pub subtitle: String,
    pub next_label: &'static str,
    pub feast_hint: Option<String>,
    pub entries: Vec<PrayerEntry>,
    pub next_idx: usize,
    pub countdown_text: String,
    pub segment_progress: f64,
    pub sun_arc_progress: f64,
}

impl PrayerSchedule {
    pub fn next_entry(&self) -> &PrayerEntry {
        &self.entries[self.next_idx.min(self.entries.len().saturating_sub(1))]
    }
}

/// 100% offline multi-faith schedule calculator for all 6 recognized traditions in Indonesia.
pub fn compute_today_schedule(cfg: &PrayerConfig) -> PrayerSchedule {
    let now = Local::now();
    let tz_hours = now.offset().local_minus_utc() as f64 / 3600.0;
    let now_min = (now.hour() * 60 + now.minute()) as i32;
    let jd = julian_day(now.year(), now.month() as i32, now.day() as i32);

    match cfg.faith {
        FaithTradition::Islam => compute_schedule_for_date(
            &cfg.city,
            cfg.latitude,
            cfg.longitude,
            &cfg.method,
            now.year(),
            now.month(),
            now.day(),
            tz_hours,
            now_min,
        ),
        other => compute_tradition_schedule(cfg, other, jd, now.weekday(), now_min),
    }
}

fn parse_hh_mm(s: &str) -> Option<i32> {
    let (h_s, m_s) = s.trim().split_once(':')?;
    let h: i32 = h_s.trim().parse().ok()?;
    let m: i32 = m_s.trim().parse().ok()?;
    if (0..=23).contains(&h) && (0..=59).contains(&m) {
        Some(h * 60 + m)
    } else {
        None
    }
}

fn compute_tradition_schedule(
    cfg: &PrayerConfig,
    faith: FaithTradition,
    jd: f64,
    weekday: chrono::Weekday,
    now_min: i32,
) -> PrayerSchedule {
    let defaults = faith.default_slots();
    let mut entries: Vec<PrayerEntry> = defaults
        .iter()
        .enumerate()
        .map(|(idx, (name, def_hhmm))| {
            let minutes = cfg
                .custom_slots
                .get(idx)
                .and_then(|s| parse_hh_mm(s))
                .or_else(|| parse_hh_mm(def_hhmm))
                .unwrap_or(6 * 60);
            PrayerEntry {
                name,
                minutes_from_midnight: minutes,
            }
        })
        .collect();

    entries.sort_by_key(|e| e.minutes_from_midnight);

    let (next_idx, countdown_text, segment_progress, sun_arc_progress) =
        compute_progress(&entries, now_min);

    let (next_label, subtitle) = match faith {
        FaithTradition::Kristen => ("Next · Saat Teduh", "Kristen Protestan".to_string()),
        FaithTradition::Katolik => ("Next · Doa Angelus", "Katolik · Angelus".to_string()),
        FaithTradition::Hindu => ("Next · Puja Tri Sandhya", "Hindu · Nusantara".to_string()),
        FaithTradition::Buddha => ("Next · Puja & Metta", "Buddha · Meditasi".to_string()),
        FaithTradition::Khonghucu => ("Next · Dupa & Renungan", "Khonghucu".to_string()),
        FaithTradition::Islam => ("Next · Kemenag RI", format!("Islam · {}", cfg.city)),
    };

    let feast_hint = if cfg.show_feast_reminder {
        Some(compute_feast_hint(faith, jd, weekday))
    } else {
        None
    };

    PrayerSchedule {
        faith,
        city: cfg.city.clone(),
        subtitle,
        next_label,
        feast_hint,
        entries,
        next_idx,
        countdown_text,
        segment_progress,
        sun_arc_progress,
    }
}

/// Offline synodic lunar phase estimator (J2000 epoch 2451550.1, synodic month 29.53058867d).
/// Returns `(days_to_new_moon, days_to_full_moon)`.
pub fn lunar_days_until_phases(jd: f64) -> (u32, u32) {
    const SYNODIC: f64 = 29.530_588_67;
    const EPOCH_NEW_MOON: f64 = 2_451_550.1;
    let age = (jd - EPOCH_NEW_MOON).rem_euclid(SYNODIC);
    let to_new = ((SYNODIC - age).rem_euclid(SYNODIC)).round() as u32;
    let full_age = SYNODIC * 0.5;
    let to_full = if age <= full_age {
        (full_age - age).round() as u32
    } else {
        (SYNODIC - age + full_age).round() as u32
    };
    (to_new, to_full)
}

fn fmt_days(days: u32) -> String {
    match days {
        0 => "Hari ini".to_string(),
        1 => "Besok".to_string(),
        d => format!("{d} hari lagi"),
    }
}

fn compute_feast_hint(faith: FaithTradition, jd: f64, weekday: chrono::Weekday) -> String {
    let days_to_sun = (7 - weekday.num_days_from_sunday()) % 7;
    let (to_new, to_full) = lunar_days_until_phases(jd);

    match faith {
        FaithTradition::Kristen => {
            if days_to_sun == 0 {
                "Ibadah Minggu · 09:00 · Hari ini".to_string()
            } else {
                format!("Ibadah Minggu · 09:00 ({})", fmt_days(days_to_sun))
            }
        }
        FaithTradition::Katolik => {
            if days_to_sun == 0 {
                "Misa Minggu · 08:00 · Hari ini".to_string()
            } else {
                format!("Misa Minggu · 08:00 ({})", fmt_days(days_to_sun))
            }
        }
        FaithTradition::Hindu => {
            if to_full <= to_new {
                format!("Rahinan · Purnama ({})", fmt_days(to_full))
            } else {
                format!("Rahinan · Tilem ({})", fmt_days(to_new))
            }
        }
        FaithTradition::Buddha => {
            if to_full <= to_new {
                format!("Uposatha · Purnama ({})", fmt_days(to_full))
            } else {
                format!("Uposatha · Bulan Baru ({})", fmt_days(to_new))
            }
        }
        FaithTradition::Khonghucu => {
            if to_full <= to_new {
                format!("Imlek · Cap Go ({})", fmt_days(to_full))
            } else {
                format!("Imlek · Che It ({})", fmt_days(to_new))
            }
        }
        FaithTradition::Islam => String::new(),
    }
}

fn compute_progress(entries: &[PrayerEntry], now_min: i32) -> (usize, String, f64, f64) {
    if entries.is_empty() {
        return (0, "in 0 min".to_string(), 0.0, 0.0);
    }
    let last_idx = entries.len() - 1;
    let next_idx = entries
        .iter()
        .position(|e| e.minutes_from_midnight > now_min)
        .unwrap_or(0);

    let next_m = entries[next_idx].minutes_from_midnight;
    let prev_m = if next_idx == 0 {
        entries[last_idx].minutes_from_midnight - 24 * 60
    } else {
        entries[next_idx - 1].minutes_from_midnight
    };
    let target_m = if next_idx == 0 && now_min >= entries[last_idx].minutes_from_midnight {
        next_m + 24 * 60
    } else {
        next_m
    };

    let diff = (target_m - now_min).max(0);
    let countdown_text = if diff >= 60 {
        format!("in {}h {}m", diff / 60, diff % 60)
    } else {
        format!("in {diff} min")
    };

    let span = (target_m - prev_m).max(1) as f64;
    let segment_progress = ((now_min - prev_m) as f64 / span).clamp(0.0, 1.0);

    let first_m = entries[0].minutes_from_midnight as f64;
    let last_m = entries[last_idx].minutes_from_midnight as f64;
    let sun_arc_progress = ((now_min as f64 - first_m) / (last_m - first_m).max(1.0)).clamp(0.0, 1.0);

    (next_idx, countdown_text, segment_progress, sun_arc_progress)
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
    let (fajr_angle, isha_angle, ihtiyati_min, method_lbl) = match method.to_ascii_lowercase().as_str() {
        "mwl" => (18.0, 17.0, 0.0, "Next · MWL"),
        "isna" => (15.0, 15.0, 0.0, "Next · ISNA"),
        "umm_al_qura" => (18.5, 18.0, 0.0, "Next · Umm al-Qura"),
        _ => (20.0, 18.0, 2.0, "Next · Kemenag RI"),
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
    let entries = vec![
        PrayerEntry { name: "Subuh", minutes_from_midnight: to_min(fajr_h, ihtiyati_min) },
        PrayerEntry { name: "Dzuhur", minutes_from_midnight: to_min(dhuhr_h, ihtiyati_min) },
        PrayerEntry { name: "Ashar", minutes_from_midnight: to_min(asr_h, ihtiyati_min) },
        PrayerEntry { name: "Maghrib", minutes_from_midnight: to_min(maghrib_h, ihtiyati_min) },
        PrayerEntry { name: "Isya", minutes_from_midnight: to_min(isha_h, ihtiyati_min) },
    ];

    let (next_idx, countdown_text, segment_progress, sun_arc_progress) =
        compute_progress(&entries, now_min);

    PrayerSchedule {
        faith: FaithTradition::Islam,
        city: city.to_string(),
        subtitle: format!("Islam · {city}"),
        next_label: method_lbl,
        feast_hint: None,
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
        assert!((235..=275).contains(&sched.entries[0].minutes_from_midnight), "Subuh {}", sched.entries[0].hh_mm());
        assert!((690..=725).contains(&sched.entries[1].minutes_from_midnight), "Dzuhur {}", sched.entries[1].hh_mm());
        assert!((885..=935).contains(&sched.entries[2].minutes_from_midnight), "Ashar {}", sched.entries[2].hh_mm());
        assert!((1060..=1100).contains(&sched.entries[3].minutes_from_midnight), "Maghrib {}", sched.entries[3].hh_mm());
        assert!((1130..=1175).contains(&sched.entries[4].minutes_from_midnight), "Isya {}", sched.entries[4].hh_mm());
        assert_eq!(sched.entries[sched.next_idx].name, "Ashar");
    }

    #[test]
    fn all_six_faiths_generate_valid_schedules() {
        let jd = julian_day(2026, 11, 26);
        for &faith in FaithTradition::ALL {
            let mut cfg = PrayerConfig::default();
            cfg.faith = faith;
            let sched = match faith {
                FaithTradition::Islam => compute_schedule_for_date("Surabaya", -7.29, 112.79, "kemenag", 2026, 11, 26, 7.0, 14 * 60),
                other => compute_tradition_schedule(&cfg, other, jd, chrono::Weekday::Thu, 17 * 60 + 15),
            };
            assert!(!sched.entries.is_empty(), "{faith:?} must have slots");
            assert!(sched.next_idx < sched.entries.len());
            if faith != FaithTradition::Islam {
                assert!(sched.feast_hint.is_some());
            }
        }
    }

    #[test]
    fn custom_slots_override_defaults() {
        let mut cfg = PrayerConfig::default();
        cfg.faith = FaithTradition::Hindu;
        cfg.custom_slots = vec!["05:30".to_string(), "12:15".to_string(), "18:30".to_string()];
        let jd = julian_day(2026, 11, 26);
        let sched = compute_tradition_schedule(&cfg, FaithTradition::Hindu, jd, chrono::Weekday::Thu, 17 * 60);
        assert_eq!(sched.entries[0].hh_mm(), "05:30");
        assert_eq!(sched.entries[1].hh_mm(), "12:15");
        assert_eq!(sched.entries[2].hh_mm(), "18:30");
        assert_eq!(sched.next_entry().name, "Sore (Sandhya)");
    }
}
