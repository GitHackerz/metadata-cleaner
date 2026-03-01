use crate::backend::models::{FileRecord, FileStatus};
use log::{debug, warn};
use tokio::sync::{mpsc, watch};
use uuid::Uuid;
use walkdir::WalkDir;

#[derive(Debug)]
pub enum ScannerMessage {
    FoundFile(FileRecord),
    Completed(i32), // Total files found
    Error(String),
}

pub struct Scanner;

impl Scanner {
    /// Scan a directory, sending messages over `tx`.
    ///
    /// Pass a `watch::Receiver<bool>` as `cancel_rx`; send `true` to abort the scan early.
    pub async fn scan_directory(
        root_path: String,
        scan_id: String,
        recursive: bool,
        tx: mpsc::Sender<ScannerMessage>,
        cancel_rx: watch::Receiver<bool>,
    ) {
        tokio::task::spawn_blocking(move || {
            let mut total_files = 0i32;
            let walker = WalkDir::new(&root_path)
                .follow_links(false)
                .max_depth(if recursive { usize::MAX } else { 1 })
                .into_iter();

            for entry in walker {
                // Check for cancellation on every entry
                if *cancel_rx.borrow() {
                    debug!("Scan cancelled by user");
                    let _ = tx.blocking_send(ScannerMessage::Error("Scan cancelled".into()));
                    return;
                }

                match entry {
                    Err(e) => {
                        warn!("Scanner: skipping inaccessible entry — {}", e);
                    }
                    Ok(entry) => {
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

                                    if tx
                                        .blocking_send(ScannerMessage::FoundFile(file_record))
                                        .is_err()
                                    {
                                        return; // Receiver dropped — app is shutting down
                                    }
                                    total_files += 1;
                                }
                            }
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
            "pdf" |                                              // Documents
            "docx" | "xlsx" | "pptx" |                         // Office
            "mp3" | "wav" | "flac" | "mp4" | "mov" | "avi" | "mkv" // Media
        )
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_image_extensions() {
        for ext in &["jpg", "jpeg", "png", "gif", "tiff", "webp"] {
            assert!(
                Scanner::is_supported_extension(ext),
                "{} should be supported",
                ext
            );
        }
    }

    #[test]
    fn supported_document_extensions() {
        for ext in &["pdf", "docx", "xlsx", "pptx"] {
            assert!(
                Scanner::is_supported_extension(ext),
                "{} should be supported",
                ext
            );
        }
    }

    #[test]
    fn supported_media_extensions() {
        for ext in &["mp3", "wav", "flac", "mp4", "mov", "avi", "mkv"] {
            assert!(
                Scanner::is_supported_extension(ext),
                "{} should be supported",
                ext
            );
        }
    }

    #[test]
    fn unsupported_extensions_rejected() {
        for ext in &["txt", "rs", "toml", "exe", "zip", "tar", "html", "xml"] {
            assert!(
                !Scanner::is_supported_extension(ext),
                "{} should NOT be supported",
                ext
            );
        }
    }

    #[test]
    fn extension_check_is_case_sensitive_lowercase_only() {
        // Our scanner lowercases extensions before checking, so uppercase should
        // fail the raw function (the lowercasing happens in scan_directory).
        assert!(!Scanner::is_supported_extension("JPG"));
        assert!(!Scanner::is_supported_extension("PDF"));
    }
}
