use anyhow::{Context, Result};
use std::fs::File;
use std::path::Path;

pub fn create_bundle(dir: &Path, output: &Path, debug: bool) -> Result<()> {
    let tar_gz = File::create(output)?;
    let enc = flate2::write::GzEncoder::new(tar_gz, flate2::Compression::default());
    let mut tar = tar::Builder::new(enc);

    if debug {
        println!("\nDebug: Files being bundled:");
        // Walk the directory and print file sizes
        for entry in walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            if let Ok(metadata) = entry.metadata() {
                let size = metadata.len();
                let relative_path = entry.path().strip_prefix(dir).unwrap_or(entry.path());
                println!("  {} - {}", relative_path.display(), format_size(size));
            }
        }
        println!();
    }

    tar.append_dir_all(".", dir)
        .context("Failed to create tar bundle")?;

    tar.finish().context("Failed to finalize tar bundle")?;

    Ok(())
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
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
