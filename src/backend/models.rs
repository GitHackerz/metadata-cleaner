use serde::{Deserialize, Serialize};
use chrono::{DateTime, Local};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanRecord {
    pub id: String,
    pub timestamp: DateTime<Local>,
    pub root_path: String,
    pub recursive: bool,
    pub total_files: i32,
    pub cleaned_files: i32,
    pub status: ScanStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScanStatus {
    InProgress,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    pub id: String,
    pub scan_id: String,
    pub path: String,
    pub file_type: String,
    pub metadata: Option<String>, // JSON string of metadata
    pub status: FileStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileStatus {
    Scanned,
    Cleaned,
    Error(String),
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub recursive_default: bool,
    pub backup_enabled: bool,
    pub theme: String, // "light" or "dark"
    pub last_scan_path: Option<String>,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            recursive_default: true,
            backup_enabled: true,
            theme: "dark".to_string(),
            last_scan_path: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_preferences_default_values() {
        let prefs = UserPreferences::default();
        assert!(prefs.recursive_default);
        assert!(prefs.backup_enabled);
        assert_eq!(prefs.theme, "dark");
        assert!(prefs.last_scan_path.is_none());
    }

    #[test]
    fn scan_status_serialization_round_trip() {
        let statuses = vec![
            ScanStatus::InProgress,
            ScanStatus::Completed,
            ScanStatus::Failed("Some error".into()),
        ];
        for status in statuses {
            let json = serde_json::to_string(&status).expect("serialize");
            let back: ScanStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(status, back);
        }
    }

    #[test]
    fn file_status_serialization_round_trip() {
        let statuses = vec![
            FileStatus::Scanned,
            FileStatus::Cleaned,
            FileStatus::Error("io error".into()),
            FileStatus::Skipped,
        ];
        for status in statuses {
            let json = serde_json::to_string(&status).expect("serialize");
            let back: FileStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(status, back);
        }
    }

    #[test]
    fn file_record_serializes_with_no_metadata() {
        let rec = FileRecord {
            id: "abc".into(),
            scan_id: "scan1".into(),
            path: "/tmp/photo.jpg".into(),
            file_type: "jpg".into(),
            metadata: None,
            status: FileStatus::Scanned,
        };
        let json = serde_json::to_string(&rec).expect("serialize");
        assert!(json.contains("\"metadata\":null"));
    }
}

