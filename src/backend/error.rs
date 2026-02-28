use thiserror::Error;

#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("ExifTool error: {0}")]
    ExifTool(String),

    #[error("ExifTool is not installed or not found on PATH. Please install ExifTool: https://exiftool.org")]
    ExifToolNotFound,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Scan error: {0}")]
    Scan(String),

    #[error("Export error: {0}")]
    Export(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
