use rusqlite::{params, Connection, Result};
use std::path::Path;
use crate::backend::models::{ScanRecord, FileRecord, UserPreferences, ScanStatus, FileStatus};
use chrono::{DateTime, Local};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.init()?;
        Ok(db)
    }

    fn init(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS scans (
                id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                root_path TEXT NOT NULL,
                recursive INTEGER NOT NULL,
                total_files INTEGER NOT NULL,
                cleaned_files INTEGER NOT NULL,
                status TEXT NOT NULL
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS files (
                id TEXT PRIMARY KEY,
                scan_id TEXT NOT NULL,
                path TEXT NOT NULL,
                file_type TEXT NOT NULL,
                metadata TEXT,
                status TEXT NOT NULL,
                FOREIGN KEY(scan_id) REFERENCES scans(id)
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS preferences (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                recursive_default INTEGER NOT NULL,
                backup_enabled INTEGER NOT NULL,
                theme TEXT NOT NULL,
                last_scan_path TEXT
            )",
            [],
        )?;

        Ok(())
    }

    pub fn save_scan(&self, scan: &ScanRecord) -> Result<()> {
        let status_str = serde_json::to_string(&scan.status).unwrap_or_default();
        self.conn.execute(
            "INSERT OR REPLACE INTO scans (id, timestamp, root_path, recursive, total_files, cleaned_files, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                scan.id,
                scan.timestamp.to_rfc3339(),
                scan.root_path,
                scan.recursive,
                scan.total_files,
                scan.cleaned_files,
                status_str
            ],
        )?;
        Ok(())
    }

    pub fn save_file(&self, file: &FileRecord) -> Result<()> {
        let status_str = serde_json::to_string(&file.status).unwrap_or_default();
        self.conn.execute(
            "INSERT OR REPLACE INTO files (id, scan_id, path, file_type, metadata, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                file.id,
                file.scan_id,
                file.path,
                file.file_type,
                file.metadata,
                status_str
            ],
        )?;
        Ok(())
    }

    pub fn get_preferences(&self) -> Result<UserPreferences> {
        let mut stmt = self.conn.prepare("SELECT recursive_default, backup_enabled, theme, last_scan_path FROM preferences WHERE id = 1")?;
        let mut rows = stmt.query([])?;

        if let Some(row) = rows.next()? {
            Ok(UserPreferences {
                recursive_default: row.get(0)?,
                backup_enabled: row.get(1)?,
                theme: row.get(2)?,
                last_scan_path: row.get(3)?,
            })
        } else {
            Ok(UserPreferences::default())
        }
    }

    pub fn save_preferences(&self, prefs: &UserPreferences) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO preferences (id, recursive_default, backup_enabled, theme, last_scan_path)
             VALUES (1, ?1, ?2, ?3, ?4)",
            params![
                prefs.recursive_default,
                prefs.backup_enabled,
                prefs.theme,
                prefs.last_scan_path
            ],
        )?;
        Ok(())
    }

    pub fn get_recent_scans(&self, limit: i32) -> Result<Vec<ScanRecord>> {
        let mut stmt = self.conn.prepare("SELECT id, timestamp, root_path, recursive, total_files, cleaned_files, status FROM scans ORDER BY timestamp DESC LIMIT ?1")?;
        let rows = stmt.query_map([limit], |row| {
            let status_str: String = row.get(6)?;
            let status: ScanStatus = serde_json::from_str(&status_str).unwrap_or(ScanStatus::Failed("Parse Error".into()));
            let timestamp_str: String = row.get(1)?;
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str).unwrap_or_default().with_timezone(&Local);

            Ok(ScanRecord {
                id: row.get(0)?,
                timestamp,
                root_path: row.get(2)?,
                recursive: row.get(3)?,
                total_files: row.get(4)?,
                cleaned_files: row.get(5)?,
                status,
            })
        })?;

        let mut scans = Vec::new();
        for scan in rows {
            scans.push(scan?);
        }
        Ok(scans)
    }
}
