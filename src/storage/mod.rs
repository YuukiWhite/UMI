pub mod secrets;

use crate::quota::QuotaSnapshot;
use anyhow::{Context, Result};
use chrono::{Local, TimeZone, Timelike};
use directories::ProjectDirs;
use rusqlite::{params, Connection, OptionalExtension};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct CachedLyrics {
    pub synced_lrc: Option<String>,
    pub plain_lyrics: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TaskItem {
    pub id: i64,
    pub title: String,
    pub completed: bool,
}

#[derive(Debug, Clone)]
pub struct ShelfItem {
    pub uri: String,
    pub name: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Default)]
pub struct WaterSummary {
    pub today_ml: u32,
    /// 8 hourly buckets starting from 09:00..=16:00 (matching FigCraft hourly bar chart)
    pub hourly_ml: [u32; 8],
}

#[derive(Clone)]
pub struct LocalDb {
    conn: Arc<Mutex<Connection>>,
}

impl LocalDb {
    pub fn open_default() -> Result<Self> {
        let path = db_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create data dir {}", parent.display()))?;
        }
        let conn = Connection::open(&path)
            .with_context(|| format!("Failed to open SQLite DB {}", path.display()))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        Self::init_schema(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn init_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS lyrics_cache (
                cache_key     TEXT PRIMARY KEY,
                synced_lrc    TEXT,
                plain_lyrics  TEXT,
                duration_sec  INTEGER,
                fetched_at    INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS tasks (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                title       TEXT NOT NULL,
                completed   INTEGER NOT NULL DEFAULT 0,
                due_at      INTEGER,
                source      TEXT NOT NULL DEFAULT 'local',
                created_at  INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS water_logs (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                amount_ml   INTEGER NOT NULL,
                logged_at   INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS shelf_items (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                uri         TEXT NOT NULL UNIQUE,
                name        TEXT NOT NULL,
                size_bytes  INTEGER NOT NULL,
                mime_type   TEXT,
                added_at    INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS quota_snapshots (
                provider_id   TEXT PRIMARY KEY,
                display_name  TEXT NOT NULL,
                plan_label    TEXT NOT NULL,
                remaining_pct INTEGER NOT NULL,
                resets_in_sec INTEGER,
                status        TEXT NOT NULL,
                accent_hex    TEXT NOT NULL,
                updated_at    INTEGER NOT NULL
            );
            ",
        )?;
        Ok(())
    }

    pub fn get_lyrics(&self, cache_key: &str) -> Result<Option<CachedLyrics>> {
        let guard = self.conn.lock().expect("db mutex poisoned");
        let row = guard
            .query_row(
                "SELECT synced_lrc, plain_lyrics FROM lyrics_cache WHERE cache_key = ?1",
                params![cache_key],
                |r| {
                    Ok(CachedLyrics {
                        synced_lrc: r.get(0)?,
                        plain_lyrics: r.get(1)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    pub fn put_lyrics(
        &self,
        cache_key: &str,
        synced_lrc: Option<&str>,
        plain_lyrics: Option<&str>,
        duration_sec: Option<i64>,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let guard = self.conn.lock().expect("db mutex poisoned");
        guard.execute(
            "INSERT INTO lyrics_cache (cache_key, synced_lrc, plain_lyrics, duration_sec, fetched_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(cache_key) DO UPDATE SET
                synced_lrc = excluded.synced_lrc,
                plain_lyrics = excluded.plain_lyrics,
                duration_sec = excluded.duration_sec,
                fetched_at = excluded.fetched_at",
            params![cache_key, synced_lrc, plain_lyrics, duration_sec, now],
        )?;
        Ok(())
    }

    // --- Water CRUD ---

    pub fn log_water(&self, amount_ml: u32) -> Result<WaterSummary> {
        let now = chrono::Utc::now().timestamp();
        {
            let guard = self.conn.lock().expect("db mutex poisoned");
            guard.execute(
                "INSERT INTO water_logs (amount_ml, logged_at) VALUES (?1, ?2)",
                params![amount_ml as i64, now],
            )?;
        }
        self.today_water_summary()
    }

    pub fn today_water_summary(&self) -> Result<WaterSummary> {
        let now = Local::now();
        let start_of_day = Local
            .from_local_datetime(&now.date_naive().and_hms_opt(0, 0, 0).expect("valid midnight"))
            .earliest()
            .map(|dt| dt.timestamp())
            .unwrap_or(0);

        let guard = self.conn.lock().expect("db mutex poisoned");
        let mut stmt = guard.prepare(
            "SELECT amount_ml, logged_at FROM water_logs WHERE logged_at >= ?1 ORDER BY logged_at ASC",
        )?;
        let rows = stmt.query_map(params![start_of_day], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))
        })?;

        let mut summary = WaterSummary::default();
        for row in rows.flatten() {
            let amt = row.0.max(0) as u32;
            summary.today_ml = summary.today_ml.saturating_add(amt);
            if let Some(dt) = Local.timestamp_opt(row.1, 0).single() {
                let h = dt.hour();
                let bucket = if h < 9 {
                    0
                } else if h >= 16 {
                    7
                } else {
                    (h - 9) as usize
                };
                summary.hourly_ml[bucket] = summary.hourly_ml[bucket].saturating_add(amt);
            }
        }
        Ok(summary)
    }

    // --- Tasks CRUD ---

    pub fn list_tasks(&self) -> Result<Vec<TaskItem>> {
        let guard = self.conn.lock().expect("db mutex poisoned");
        let mut stmt = guard.prepare(
            "SELECT id, title, completed FROM tasks ORDER BY completed ASC, id DESC LIMIT 30",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(TaskItem {
                id: r.get(0)?,
                title: r.get(1)?,
                completed: r.get::<_, i64>(2)? != 0,
            })
        })?;
        Ok(rows.flatten().collect())
    }

    pub fn add_task(&self, title: &str) -> Result<Vec<TaskItem>> {
        let clean = title.trim();
        if !clean.is_empty() {
            let now = chrono::Utc::now().timestamp();
            let guard = self.conn.lock().expect("db mutex poisoned");
            guard.execute(
                "INSERT INTO tasks (title, completed, source, created_at) VALUES (?1, 0, 'local', ?2)",
                params![clean, now],
            )?;
        }
        self.list_tasks()
    }

    pub fn toggle_task(&self, id: i64) -> Result<Vec<TaskItem>> {
        {
            let guard = self.conn.lock().expect("db mutex poisoned");
            guard.execute(
                "UPDATE tasks SET completed = CASE WHEN completed = 0 THEN 1 ELSE 0 END WHERE id = ?1",
                params![id],
            )?;
        }
        self.list_tasks()
    }

    pub fn delete_task(&self, id: i64) -> Result<Vec<TaskItem>> {
        {
            let guard = self.conn.lock().expect("db mutex poisoned");
            guard.execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
        }
        self.list_tasks()
    }

    // --- Shelf CRUD ---

    pub fn list_shelf_items(&self) -> Result<Vec<ShelfItem>> {
        let guard = self.conn.lock().expect("db mutex poisoned");
        let mut stmt = guard.prepare(
            "SELECT uri, name, size_bytes FROM shelf_items ORDER BY added_at DESC LIMIT 12",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(ShelfItem {
                uri: r.get(0)?,
                name: r.get(1)?,
                size_bytes: r.get::<_, i64>(2)?.max(0) as u64,
            })
        })?;
        Ok(rows.flatten().collect())
    }

    pub fn add_shelf_item(&self, uri: &str, name: &str, size_bytes: u64) -> Result<Vec<ShelfItem>> {
        let now = chrono::Utc::now().timestamp();
        {
            let guard = self.conn.lock().expect("db mutex poisoned");
            guard.execute(
                "INSERT INTO shelf_items (uri, name, size_bytes, added_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(uri) DO UPDATE SET name = excluded.name, size_bytes = excluded.size_bytes, added_at = excluded.added_at",
                params![uri, name, size_bytes as i64, now],
            )?;
        }
        self.list_shelf_items()
    }

    pub fn clear_shelf(&self) -> Result<Vec<ShelfItem>> {
        {
            let guard = self.conn.lock().expect("db mutex poisoned");
            guard.execute("DELETE FROM shelf_items", [])?;
        }
        Ok(Vec::new())
    }

    // --- Quota Snapshots Cache ---

    pub fn save_quota_snapshots(&self, snaps: &[QuotaSnapshot]) -> Result<()> {
        let guard = self.conn.lock().expect("db mutex poisoned");
        for s in snaps {
            let status_s = format!("{:?}", s.status).to_ascii_lowercase();
            guard.execute(
                "INSERT INTO quota_snapshots (provider_id, display_name, plan_label, remaining_pct, resets_in_sec, status, accent_hex, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(provider_id) DO UPDATE SET
                    display_name = excluded.display_name,
                    plan_label = excluded.plan_label,
                    remaining_pct = excluded.remaining_pct,
                    resets_in_sec = excluded.resets_in_sec,
                    status = excluded.status,
                    accent_hex = excluded.accent_hex,
                    updated_at = excluded.updated_at",
                params![
                    s.provider_id,
                    s.display_name,
                    s.plan_label,
                    s.remaining_pct as i64,
                    s.resets_in_sec.map(|v| v as i64),
                    status_s,
                    s.accent_hex,
                    s.updated_at
                ],
            )?;
        }
        Ok(())
    }
}

pub fn db_path() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("org", "umi", "umi")
        .context("Failed to resolve XDG data directory")?;
    Ok(dirs.data_dir().join("umi.db"))
}

pub fn art_cache_dir() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("org", "umi", "umi")
        .context("Failed to resolve XDG cache directory")?;
    let dir = dirs.cache_dir().join("art");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}
