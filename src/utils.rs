use anyhow::{Context, Result};
use std::fs::File;
use std::path::Path;

pub fn create_bundle(dir: &Path, output: &Path) -> Result<()> {
    let tar_gz = File::create(output)?;
    let enc = flate2::write::GzEncoder::new(tar_gz, flate2::Compression::default());
    let mut tar = tar::Builder::new(enc);

    tar.append_dir_all(".", dir)
        .context("Failed to create tar bundle")?;

    tar.finish().context("Failed to finalize tar bundle")?;

    Ok(())
}

pub fn format_timestamp(timestamp: &str) -> String {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(timestamp) {
        dt.format("%Y-%m-%d %H:%M:%S").to_string()
    } else {
        timestamp.to_string()
    }
}

pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

pub fn confirm(message: &str) -> Result<bool> {
    use dialoguer::Confirm;

    Ok(Confirm::new().with_prompt(message).interact()?)
}
