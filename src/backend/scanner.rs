use walkdir::WalkDir;
use std::path::Path;
use tokio::sync::mpsc;
use crate::backend::models::{FileRecord, FileStatus};
use uuid::Uuid;

#[derive(Debug)]
pub enum ScannerMessage {
    FoundFile(FileRecord),
    Completed(i32), // Total files found
    Error(String),
}

pub struct Scanner;

impl Scanner {
    pub async fn scan_directory(
        root_path: String,
        scan_id: String,
        recursive: bool,
        tx: mpsc::Sender<ScannerMessage>,
    ) {
        tokio::task::spawn_blocking(move || {
            let mut total_files = 0;
            let walker = WalkDir::new(&root_path)
                .max_depth(if recursive { usize::MAX } else { 1 })
                .into_iter();

            for entry in walker.filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        let ext_str = ext.to_string_lossy().to_lowercase();
                        if Self::is_supported_extension(&ext_str) {
                            let file_record = FileRecord {
                                id: Uuid::new_v4().to_string(),
                                scan_id: scan_id.clone(),
                                path: path.to_string_lossy().to_string(),
                                file_type: ext_str,
                                metadata: None,
                                status: FileStatus::Scanned,
                            };

                            if tx.blocking_send(ScannerMessage::FoundFile(file_record)).is_err() {
                                break;
                            }
                            total_files += 1;
                        }
                    }
                }
            }

            let _ = tx.blocking_send(ScannerMessage::Completed(total_files));
        });
    }

    fn is_supported_extension(ext: &str) -> bool {
        matches!(
            ext,
            "jpg" | "jpeg" | "png" | "gif" | "tiff" | "webp" | // Images
            "pdf" | // Documents
            "docx" | "xlsx" | "pptx" | // Office
            "mp3" | "wav" | "flac" | "mp4" | "mov" | "avi" | "mkv" // Media
        )
    }
}
