use std::process::Command;
use serde_json::Value;
use anyhow::{Result, anyhow};
use std::fs;

pub struct ExifTool;

impl ExifTool {
    pub fn check_availability() -> bool {
        Command::new("exiftool")
            .arg("-ver")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[allow(dead_code)]
    pub fn get_metadata(path: &str) -> Result<Value> {
        let output = Command::new("exiftool")
            .arg("-j") // JSON output
            .arg("-g") // Group by tag group
            .arg(path)
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("ExifTool failed: {}", String::from_utf8_lossy(&output.stderr)));
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let json: Vec<Value> = serde_json::from_str(&json_str)?;

        json.first().cloned().ok_or_else(|| anyhow!("No metadata found"))
    }

    pub fn clean_metadata(path: &str, backup: bool) -> Result<()> {
        let mut cmd = Command::new("exiftool");
        cmd.arg("-all="); // Remove all tags
        
        if !backup {
            cmd.arg("-overwrite_original");
        }

        cmd.arg(path);

        let output = cmd.output()?;

        if !output.status.success() {
            return Err(anyhow!("Failed to clean metadata: {}", String::from_utf8_lossy(&output.stderr)));
        }

        Ok(())
    }

    pub fn create_backup(path: &str) -> Result<String> {
        let backup_path = format!("{}.original", path);
        fs::copy(path, &backup_path)?;
        Ok(backup_path)
    }
}
