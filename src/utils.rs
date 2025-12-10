use anyhow::{Context, Result};
use globset::{Glob, GlobSetBuilder};
use std::fs::File;
use std::path::{Path, PathBuf};

/// Prepare a list of files to bundle based on include/exclude patterns
///
/// Logic:
/// 1. Always exclude .venv and .renv directories
/// 2. If include patterns are specified, ONLY include paths matching those patterns
/// 3. Then exclude any paths matching the exclude patterns
/// 4. Otherwise include everything (except blacklisted directories)
pub fn prepare_bundle(
    dir: &Path,
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
) -> Result<Vec<PathBuf>> {
    // prevent including virtual environments and renv caches
    let mut blacklist_builder = GlobSetBuilder::new();
    blacklist_builder.add(Glob::new(".venv")?);
    blacklist_builder.add(Glob::new(".venv/**")?);
    blacklist_builder.add(Glob::new(".renv")?);
    blacklist_builder.add(Glob::new(".renv/**")?);
    let blacklist = blacklist_builder.build()?;

    let include_matcher = if let Some(patterns) = include {
        let mut builder = GlobSetBuilder::new();
        for pattern in patterns {
            builder.add(Glob::new(&pattern)?);
        }
        Some(builder.build()?)
    } else {
        None
    };

    let exclude_matcher = if let Some(patterns) = exclude {
        let mut builder = GlobSetBuilder::new();
        for pattern in patterns {
            builder.add(Glob::new(&pattern)?);
        }
        Some(builder.build()?)
    } else {
        None
    };

    let mut files_to_bundle = Vec::new();

    for entry in walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let relative_path = entry.path().strip_prefix(dir).unwrap_or(entry.path());

        if blacklist.is_match(relative_path) {
            continue;
        }

        // include / exclude files based on include
        if let Some(ref matcher) = include_matcher
            && !matcher.is_match(relative_path)
        {
            continue;
        }

        if let Some(ref matcher) = exclude_matcher
            && matcher.is_match(relative_path)
        {
            continue;
        }

        files_to_bundle.push(entry.path().to_path_buf());
    }

    Ok(files_to_bundle)
}

pub fn create_bundle(
    dir: &Path,
    output: &Path,
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
    debug: bool,
) -> Result<()> {
    let tar_gz = File::create(output)?;
    let enc = flate2::write::GzEncoder::new(tar_gz, flate2::Compression::default());
    let mut tar = tar::Builder::new(enc);

    let files_to_bundle = prepare_bundle(dir, include, exclude)?;

    if debug {
        println!("\nDebug: Files being bundled:");
        for path in &files_to_bundle {
            if path.is_file()
                && let Ok(metadata) = std::fs::metadata(path)
            {
                let size = metadata.len();
                let relative_path = path.strip_prefix(dir).unwrap_or(path);
                println!("  {} - {}", relative_path.display(), format_size(size));
            }
        }
        println!();
    }

    // Add files to tar (directories will be created automatically)
    for path in files_to_bundle {
        // Only add files, skip directories
        if !path.is_file() {
            continue;
        }

        let relative_path = path.strip_prefix(dir).unwrap_or(&path);

        // Skip empty paths (root directory)
        if relative_path.as_os_str().is_empty() {
            continue;
        }

        tar.append_path_with_name(&path, relative_path)
            .context(format!(
                "Failed to add {} to bundle",
                relative_path.display()
            ))?;
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_prepare_bundle_excludes_venv() {
        let temp_dir = tempdir().unwrap();
        let dir_path = temp_dir.path();

        // Create test files and directories
        fs::write(dir_path.join(".python-version"), "3.11").unwrap();
        fs::create_dir(dir_path.join(".venv")).unwrap();
        fs::write(dir_path.join(".venv").join("pyvenv.cfg"), "config").unwrap();
        fs::write(dir_path.join("main.py"), "print('hello')").unwrap();
        fs::write(dir_path.join("uv.lock"), "lock file").unwrap();
        fs::write(dir_path.join("pyproject.toml"), "[project]").unwrap();

        // Run prepare_bundle with no include/exclude patterns
        let result = prepare_bundle(dir_path, None, None).unwrap();

        // Convert results to relative paths for easier assertion
        let relative_paths: Vec<String> = result
            .iter()
            .map(|p| {
                p.strip_prefix(dir_path)
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        // Verify .venv is NOT included
        assert!(
            !relative_paths.iter().any(|p| p.starts_with(".venv")),
            "Bundle should not contain .venv directory or its contents"
        );

        // Verify expected files ARE included
        assert!(relative_paths.contains(&".python-version".to_string()));
        assert!(relative_paths.contains(&"main.py".to_string()));
        assert!(relative_paths.contains(&"uv.lock".to_string()));
        assert!(relative_paths.contains(&"pyproject.toml".to_string()));
    }

    #[test]
    fn test_prepare_bundle_excludes_renv() {
        let temp_dir = tempdir().unwrap();
        let dir_path = temp_dir.path();

        // Create test files and directories
        fs::write(dir_path.join("app.R"), "library(shiny)").unwrap();
        fs::create_dir(dir_path.join(".renv")).unwrap();
        fs::write(dir_path.join(".renv").join("activate.R"), "source").unwrap();
        fs::write(dir_path.join("renv.lock"), "lock file").unwrap();

        // Run prepare_bundle with no include/exclude patterns
        let result = prepare_bundle(dir_path, None, None).unwrap();

        // Convert results to relative paths for easier assertion
        let relative_paths: Vec<String> = result
            .iter()
            .map(|p| {
                p.strip_prefix(dir_path)
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        // Verify .renv is NOT included
        assert!(
            !relative_paths.iter().any(|p| p.starts_with(".renv")),
            "Bundle should not contain .renv directory or its contents"
        );

        // Verify expected files ARE included
        assert!(relative_paths.contains(&"app.R".to_string()));
        assert!(relative_paths.contains(&"renv.lock".to_string()));
    }

    #[test]
    fn test_prepare_bundle_excludes_venv_even_when_included() {
        let temp_dir = tempdir().unwrap();
        let dir_path = temp_dir.path();

        // Create test files and directories
        fs::write(dir_path.join(".python-version"), "3.11").unwrap();
        fs::create_dir(dir_path.join(".venv")).unwrap();
        fs::write(dir_path.join(".venv").join("pyvenv.cfg"), "config").unwrap();
        fs::write(dir_path.join("main.py"), "print('hello')").unwrap();
        fs::write(dir_path.join("pyproject.toml"), "[project]").unwrap();

        // Try to explicitly include .venv in the include patterns
        let include = Some(vec![
            ".venv".to_string(),
            ".venv/**".to_string(),
            "**/*.py".to_string(),
        ]);

        // Run prepare_bundle with include pattern that includes .venv
        let result = prepare_bundle(dir_path, include, None).unwrap();

        // Convert results to relative paths for easier assertion
        let relative_paths: Vec<String> = result
            .iter()
            .map(|p| {
                p.strip_prefix(dir_path)
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        // Verify .venv is STILL NOT included despite being in include patterns
        assert!(
            !relative_paths.iter().any(|p| p.starts_with(".venv")),
            "Bundle should not contain .venv directory even when explicitly included"
        );

        // Verify main.py is included (matches **/*.py pattern)
        assert!(relative_paths.contains(&"main.py".to_string()));
    }

    #[test]
    fn test_prepare_bundle_excludes_renv_even_when_included() {
        let temp_dir = tempdir().unwrap();
        let dir_path = temp_dir.path();

        // Create test files and directories
        fs::write(dir_path.join("app.R"), "library(shiny)").unwrap();
        fs::create_dir(dir_path.join(".renv")).unwrap();
        fs::write(dir_path.join(".renv").join("activate.R"), "source").unwrap();
        fs::write(dir_path.join("renv.lock"), "lock file").unwrap();

        // Try to explicitly include .renv in the include patterns
        let include = Some(vec![
            ".renv".to_string(),
            ".renv/**".to_string(),
            "**/*.R".to_string(),
        ]);

        // Run prepare_bundle with include pattern that includes .renv
        let result = prepare_bundle(dir_path, include, None).unwrap();

        // Convert results to relative paths for easier assertion
        let relative_paths: Vec<String> = result
            .iter()
            .map(|p| {
                p.strip_prefix(dir_path)
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        // Verify .renv is STILL NOT included despite being in include patterns
        assert!(
            !relative_paths.iter().any(|p| p.starts_with(".renv")),
            "Bundle should not contain .renv directory even when explicitly included"
        );

        // Verify app.R is included (matches **/*.R pattern)
        assert!(relative_paths.contains(&"app.R".to_string()));
    }
}
