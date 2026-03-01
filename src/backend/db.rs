use crate::backend::models::{FileRecord, FileStatus, ScanRecord, ScanStatus, UserPreferences};
use chrono::{DateTime, Local};
use rusqlite::{params, Connection, Result};
use std::path::Path;
use std::sync::Mutex;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init()?;
        Ok(db)
    }

    fn init(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        // Enable WAL mode for better concurrent read performance
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

        conn.execute(
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

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_scans_timestamp ON scans(timestamp DESC)",
            [],
        )?;

        conn.execute(
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

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_files_scan_id ON files(scan_id)",
            [],
        )?;

        conn.execute(
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
        let conn = self.conn.lock().unwrap();
        let status_str = serde_json::to_string(&scan.status).unwrap_or_default();
        conn.execute(
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
        let conn = self.conn.lock().unwrap();
        let status_str = serde_json::to_string(&file.status).unwrap_or_default();
        conn.execute(
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
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT recursive_default, backup_enabled, theme, last_scan_path FROM preferences WHERE id = 1")?;
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
        let conn = self.conn.lock().unwrap();
        conn.execute(
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
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, root_path, recursive, total_files, cleaned_files, status \
             FROM scans ORDER BY timestamp DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit], |row| {
            let status_str: String = row.get(6)?;
            let status: ScanStatus = serde_json::from_str(&status_str)
                .unwrap_or(ScanStatus::Failed("Parse Error".into()));
            let timestamp_str: String = row.get(1)?;
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map(|dt| dt.with_timezone(&Local))
                .unwrap_or_else(|_| Local::now());

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

    /// Update only the status of an existing scan record.
    pub fn update_scan_status(&self, scan_id: &str, status: &ScanStatus) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let status_str = serde_json::to_string(status).unwrap_or_default();
        conn.execute(
            "UPDATE scans SET status = ?1 WHERE id = ?2",
            params![status_str, scan_id],
        )?;
        Ok(())
    }

    /// Update total_files and cleaned_files counts for a scan.
    pub fn update_scan_totals(
        &self,
        scan_id: &str,
        total_files: i32,
        cleaned_files: i32,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE scans SET total_files = ?1, cleaned_files = ?2 WHERE id = ?3",
            params![total_files, cleaned_files, scan_id],
        )?;
        Ok(())
    }

    /// Update the status (and optionally the metadata JSON) of a single file record.
    pub fn update_file_status(
        &self,
        file_id: &str,
        status: &FileStatus,
        metadata: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let status_str = serde_json::to_string(status).unwrap_or_default();
        conn.execute(
            "UPDATE files SET status = ?1, metadata = COALESCE(?2, metadata) WHERE id = ?3",
            params![status_str, metadata, file_id],
        )?;
        Ok(())
    }

    /// Retrieve all file records associated with a scan.
    pub fn get_files_for_scan(&self, scan_id: &str) -> Result<Vec<FileRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, scan_id, path, file_type, metadata, status \
             FROM files WHERE scan_id = ?1 ORDER BY path",
        )?;
        let rows = stmt.query_map([scan_id], |row| {
            let status_str: String = row.get(5)?;
            let status: FileStatus = serde_json::from_str(&status_str)
                .unwrap_or(FileStatus::Error("Parse Error".into()));
            Ok(FileRecord {
                id: row.get(0)?,
                scan_id: row.get(1)?,
                path: row.get(2)?,
                file_type: row.get(3)?,
                metadata: row.get(4)?,
                status,
            })
        })?;

        let mut files = Vec::new();
        for file in rows {
            files.push(file?);
        }
        Ok(files)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::models::{FileStatus, ScanStatus, UserPreferences};
    use chrono::Local;
    use uuid::Uuid;

    fn make_db() -> Database {
        // Use an in-memory SQLite database for tests
        let conn = Connection::open_in_memory().expect("open in-memory DB");
        let db = Database {
            conn: Mutex::new(conn),
        };
        db.init().expect("init schema");
        db
    }

    fn make_scan(root: &str) -> ScanRecord {
        ScanRecord {
            id: Uuid::new_v4().to_string(),
            timestamp: Local::now(),
            root_path: root.to_string(),
            recursive: true,
            total_files: 0,
            cleaned_files: 0,
            status: ScanStatus::InProgress,
        }
    }

    fn make_file(scan_id: &str, path: &str) -> FileRecord {
        FileRecord {
            id: Uuid::new_v4().to_string(),
            scan_id: scan_id.to_string(),
            path: path.to_string(),
            file_type: "jpg".to_string(),
            metadata: None,
            status: FileStatus::Scanned,
        }
    }

    #[test]
    fn save_and_retrieve_scan() {
        let db = make_db();
        let scan = make_scan("/tmp/photos");

        db.save_scan(&scan).expect("save scan");

        let scans = db.get_recent_scans(10).expect("get scans");
        assert_eq!(scans.len(), 1);
        assert_eq!(scans[0].id, scan.id);
        assert_eq!(scans[0].root_path, "/tmp/photos");
        assert_eq!(scans[0].status, ScanStatus::InProgress);
    }

    #[test]
    fn update_scan_status() {
        let db = make_db();
        let scan = make_scan("/tmp/docs");
        db.save_scan(&scan).expect("save scan");

        db.update_scan_status(&scan.id, &ScanStatus::Completed)
            .expect("update status");

        let scans = db.get_recent_scans(10).expect("get scans");
        assert_eq!(scans[0].status, ScanStatus::Completed);
    }

    #[test]
    fn update_scan_totals() {
        let db = make_db();
        let scan = make_scan("/tmp/music");
        db.save_scan(&scan).expect("save scan");

        db.update_scan_totals(&scan.id, 42, 10)
            .expect("update totals");

        let scans = db.get_recent_scans(1).expect("get scans");
        assert_eq!(scans[0].total_files, 42);
        assert_eq!(scans[0].cleaned_files, 10);
    }

    #[test]
    fn save_and_retrieve_file() {
        let db = make_db();
        let scan = make_scan("/tmp/pics");
        db.save_scan(&scan).expect("save scan");

        let file = make_file(&scan.id, "/tmp/pics/image.jpg");
        db.save_file(&file).expect("save file");

        let files = db.get_files_for_scan(&scan.id).expect("get files");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "/tmp/pics/image.jpg");
        assert_eq!(files[0].status, FileStatus::Scanned);
    }

    #[test]
    fn update_file_status() {
        let db = make_db();
        let scan = make_scan("/tmp/vids");
        db.save_scan(&scan).expect("save scan");

        let file = make_file(&scan.id, "/tmp/vids/clip.mp4");
        db.save_file(&file).expect("save file");

        db.update_file_status(&file.id, &FileStatus::Cleaned, None)
            .expect("update file status");

        let files = db.get_files_for_scan(&scan.id).expect("get files");
        assert_eq!(files[0].status, FileStatus::Cleaned);
    }

    #[test]
    fn preferences_default_when_empty() {
        let db = make_db();
        let prefs = db.get_preferences().expect("get prefs");
        // Should return defaults when no row exists
        assert!(prefs.recursive_default);
        assert_eq!(prefs.theme, "dark");
    }

    #[test]
    fn save_and_load_preferences() {
        let db = make_db();
        let prefs = UserPreferences {
            recursive_default: false,
            backup_enabled: false,
            theme: "light".to_string(),
            last_scan_path: Some("/home/user/Desktop".to_string()),
        };
        db.save_preferences(&prefs).expect("save prefs");

        let loaded = db.get_preferences().expect("get prefs");
        assert!(!loaded.recursive_default);
        assert!(!loaded.backup_enabled);
        assert_eq!(loaded.theme, "light");
        assert_eq!(loaded.last_scan_path.as_deref(), Some("/home/user/Desktop"));
    }

    #[test]
    fn get_recent_scans_respects_limit() {
        let db = make_db();
        for i in 0..5 {
            let mut scan = make_scan(&format!("/tmp/dir{}", i));
            scan.status = ScanStatus::Completed;
            db.save_scan(&scan).expect("save scan");
        }
        let scans = db.get_recent_scans(3).expect("get scans");
        assert_eq!(scans.len(), 3);
    }
}
