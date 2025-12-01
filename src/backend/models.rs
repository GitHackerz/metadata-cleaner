use serde::{Deserialize, Serialize};
use chrono::{DateTime, Local};
use uuid::Uuid;

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
