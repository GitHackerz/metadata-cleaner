use crate::backend::models::FileRecord;
use anyhow::Result;
use std::fs::File;
use std::path::Path;

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

pub fn generate_default_filename(extension: &str) -> String {
    let now = chrono::Local::now();
    format!("scan_report_{}.{}", now.format("%Y%m%d_%H%M%S"), extension)
}
