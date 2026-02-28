use crate::backend::models::FileRecord;
use anyhow::Result;
use std::fs::File;

pub fn export_json(files: &[FileRecord], path: &str) -> Result<()> {
    let file = File::create(path)?;
    serde_json::to_writer_pretty(file, files)?;
    Ok(())
}

pub fn export_csv(files: &[FileRecord], path: &str) -> Result<()> {
    let mut wtr = csv::Writer::from_path(path)?;
    for file in files {
        wtr.serialize(file)?;
    }
    wtr.flush()?;
    Ok(())
}

/// Generate a timestamped filename for a report, e.g. `scan_report_20260228_143000.json`.
pub fn generate_default_filename(extension: &str) -> String {
    let now = chrono::Local::now();
    format!("scan_report_{}.{}", now.format("%Y%m%d_%H%M%S"), extension)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::models::{FileRecord, FileStatus};
    use std::fs;

    fn sample_files() -> Vec<FileRecord> {
        vec![
            FileRecord {
                id: "1".into(),
                scan_id: "scan1".into(),
                path: "/tmp/photo.jpg".into(),
                file_type: "jpg".into(),
                metadata: None,
                status: FileStatus::Scanned,
            },
            FileRecord {
                id: "2".into(),
                scan_id: "scan1".into(),
                path: "/tmp/video.mp4".into(),
                file_type: "mp4".into(),
                metadata: Some("{\"GPS\":\"51.5,-0.1\"}".into()),
                status: FileStatus::Cleaned,
            },
        ]
    }

    #[test]
    fn export_json_creates_valid_json() {
        let tmp_path = std::env::temp_dir().join("mc_test_export.json");
        export_json(&sample_files(), tmp_path.to_str().unwrap()).expect("export json");
        let content = fs::read_to_string(&tmp_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).expect("valid json");
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 2);
        let _ = fs::remove_file(tmp_path);
    }

    #[test]
    fn export_json_empty_produces_empty_array() {
        let tmp_path = std::env::temp_dir().join("mc_test_empty.json");
        export_json(&[], tmp_path.to_str().unwrap()).expect("export empty");
        let content = fs::read_to_string(&tmp_path).unwrap();
        assert_eq!(content.trim(), "[]");
        let _ = fs::remove_file(tmp_path);
    }

    #[test]
    fn export_csv_creates_parseable_csv() {
        let tmp_path = std::env::temp_dir().join("mc_test_export.csv");
        export_csv(&sample_files(), tmp_path.to_str().unwrap()).expect("export csv");
        let content = fs::read_to_string(&tmp_path).unwrap();
        // header row + 2 data rows
        let lines: Vec<&str> = content.lines().collect();
        assert!(lines.len() >= 3, "expected header + 2 rows, got {}", lines.len());
        let _ = fs::remove_file(tmp_path);
    }

    #[test]
    fn generate_default_filename_has_correct_format() {
        let json_name = generate_default_filename("json");
        assert!(json_name.starts_with("scan_report_"));
        assert!(json_name.ends_with(".json"));

        let csv_name = generate_default_filename("csv");
        assert!(csv_name.ends_with(".csv"));
    }
}
