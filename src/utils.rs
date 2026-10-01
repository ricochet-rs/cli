use anyhow::{Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::fs::File;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

/// Check if we're running in a non-interactive environment (tests, CI, etc.)
///
/// This function checks multiple indicators to reliably detect non-interactive environments:
/// - stdin is not a terminal
/// - CI environment variable is set
/// - RUST_TEST_THREADS is set (cargo test parallel execution)
/// - CARGO_MANIFEST_DIR is set (cargo test environment)
/// - RICOCHET_NON_INTERACTIVE is explicitly set
pub fn is_non_interactive() -> bool {
    !std::io::stdin().is_terminal()
        || std::env::var("CI").is_ok()
        || std::env::var("RUST_TEST_THREADS").is_ok()
        || std::env::var("CARGO_MANIFEST_DIR").is_ok()
        || std::env::var("RICOCHET_NON_INTERACTIVE").is_ok()
}

/// Why a file stays out of the bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Exclusion {
    /// Under `.venv`, `.renv` or `__pycache__`, which are never bundled.
    AlwaysExcluded,
    /// Matched by none of the `content.include` patterns.
    NotIncluded,
    /// Matched the named `content.exclude` pattern.
    Pattern(String),
}

impl std::fmt::Display for Exclusion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlwaysExcluded => f.write_str("always-excluded"),
            Self::NotIncluded => f.write_str("not-included"),
            Self::Pattern(pattern) => write!(f, "exclude:{pattern}"),
        }
    }
}

impl serde::Serialize for Exclusion {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// A file under the bundled directory and the reason it is left out, if it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleCandidate {
    /// Path relative to the bundled directory.
    pub path: PathBuf,
    pub exclusion: Option<Exclusion>,
}

fn glob_set<'a>(patterns: impl IntoIterator<Item = &'a str>) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(Glob::new(pattern)?);
    }
    Ok(builder.build()?)
}

/// Classify every file under `dir` against the bundle rules, lazily as the walk proceeds.
///
/// Logic:
/// 1. Always exclude .venv, .renv and __pycache__ directories
/// 2. If include patterns are specified, ONLY include paths matching those patterns
/// 3. Then exclude any paths matching the exclude patterns
/// 4. Otherwise include everything (except blacklisted directories)
pub fn classify_bundle(
    dir: &Path,
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
) -> Result<impl Iterator<Item = BundleCandidate> + '_> {
    // prevent including virtual environments, renv caches, and Python bytecode caches
    // __pycache__ can appear at any nesting level, so match it recursively
    let blacklist = glob_set([
        ".venv",
        ".venv/**",
        ".renv",
        ".renv/**",
        "__pycache__",
        "__pycache__/**",
        "**/__pycache__",
        "**/__pycache__/**",
    ])?;
    let include_matcher = include
        .map(|patterns| glob_set(patterns.iter().map(String::as_str)))
        .transpose()?;
    let exclude = exclude.unwrap_or_default();
    let exclude_matcher = glob_set(exclude.iter().map(String::as_str))?;

    Ok(walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .map(move |entry| {
            let path = entry
                .path()
                .strip_prefix(dir)
                .unwrap_or(entry.path())
                .to_path_buf();

            let exclusion = if blacklist.is_match(&path) {
                Some(Exclusion::AlwaysExcluded)
            } else if include_matcher
                .as_ref()
                .is_some_and(|matcher| !matcher.is_match(&path))
            {
                Some(Exclusion::NotIncluded)
            } else {
                exclude_matcher
                    .matches(&path)
                    .first()
                    .and_then(|&index| exclude.get(index))
                    .map(|pattern| Exclusion::Pattern(pattern.clone()))
            };

            BundleCandidate { path, exclusion }
        }))
}

/// List the files a deploy bundles from `dir`, as absolute paths.
pub fn prepare_bundle(
    dir: &Path,
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
) -> Result<Vec<PathBuf>> {
    Ok(classify_bundle(dir, include, exclude)?
        .filter(|candidate| candidate.exclusion.is_none())
        .map(|candidate| dir.join(candidate.path))
        .collect())
}

pub fn create_bundle(
    dir: &Path,
    output: &Path,
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
    extra_root_files: &[(PathBuf, String)],
    debug: bool,
) -> Result<()> {
    let tar_gz = File::create(output)?;
    let enc = flate2::write::GzEncoder::new(tar_gz, flate2::Compression::default());
    let mut tar = tar::Builder::new(enc);

    let files_to_bundle = prepare_bundle(dir, include, exclude)?;

    if debug {
        eprintln!("\nDebug: Files being bundled:");
        for path in &files_to_bundle {
            if path.is_file()
                && let Ok(metadata) = std::fs::metadata(path)
            {
                let size = metadata.len();
                let relative_path = path.strip_prefix(dir).unwrap_or(path);
                eprintln!("  {} - {}", relative_path.display(), format_size(size));
            }
        }
        for (source, name) in extra_root_files {
            if let Ok(metadata) = std::fs::metadata(source) {
                eprintln!(
                    "  {} (from {}) - {}",
                    name,
                    source.display(),
                    format_size(metadata.len())
                );
            }
        }
        eprintln!();
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

    // Add extra files at the bundle root (e.g. uv.lock from a parent directory)
    for (source, name) in extra_root_files {
        tar.append_path_with_name(source, name)
            .context(format!("Failed to add {} to bundle", source.display()))?;
    }

    tar.finish().context("Failed to finalize tar bundle")?;

    Ok(())
}

pub(crate) fn format_size(bytes: u64) -> String {
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

/// Search parent directories for a file, stopping at the filesystem root.
pub fn find_in_parent_dirs(start: &Path, filename: &str) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let candidate = current.join(filename);
        if candidate.exists() {
            return Some(candidate);
        }
        if !current.pop() {
            return None;
        }
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
    fn classify_bundle_names_the_reason_for_each_exclusion() {
        let temp_dir = tempdir().unwrap();
        let dir_path = temp_dir.path();

        fs::write(dir_path.join("app.R"), "library(shiny)").unwrap();
        fs::write(dir_path.join("notes.txt"), "notes").unwrap();
        fs::create_dir_all(dir_path.join("data")).unwrap();
        fs::write(dir_path.join("data").join("big.parquet"), "rows").unwrap();
        fs::create_dir_all(dir_path.join(".venv").join("lib")).unwrap();
        fs::write(dir_path.join(".venv").join("lib").join("x.py"), "").unwrap();

        let include = Some(vec!["**/*.R".to_string(), "data/**".to_string()]);
        let exclude = Some(vec!["*.md".to_string(), "data/**".to_string()]);
        let mut result: Vec<_> = classify_bundle(dir_path, include, exclude)
            .unwrap()
            .collect();
        result.sort_by(|a, b| a.path.cmp(&b.path));

        let verdicts: Vec<(String, Option<String>)> = result
            .iter()
            .map(|c| {
                (
                    c.path.to_string_lossy().replace('\\', "/"),
                    c.exclusion.as_ref().map(Exclusion::to_string),
                )
            })
            .collect();
        assert_eq!(
            verdicts,
            vec![
                (
                    ".venv/lib/x.py".to_string(),
                    Some("always-excluded".to_string())
                ),
                ("app.R".to_string(), None),
                (
                    "data/big.parquet".to_string(),
                    Some("exclude:data/**".to_string())
                ),
                ("notes.txt".to_string(), Some("not-included".to_string())),
            ]
        );
    }

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
    fn test_prepare_bundle_excludes_pycache() {
        let temp_dir = tempdir().unwrap();
        let dir_path = temp_dir.path();

        // Create test files and a top-level plus a nested __pycache__ directory
        fs::write(dir_path.join("main.py"), "print('hello')").unwrap();
        fs::create_dir(dir_path.join("__pycache__")).unwrap();
        fs::write(
            dir_path.join("__pycache__").join("main.cpython-311.pyc"),
            "bytecode",
        )
        .unwrap();
        fs::create_dir(dir_path.join("pkg")).unwrap();
        fs::write(dir_path.join("pkg").join("mod.py"), "x = 1").unwrap();
        fs::create_dir(dir_path.join("pkg").join("__pycache__")).unwrap();
        fs::write(
            dir_path
                .join("pkg")
                .join("__pycache__")
                .join("mod.cpython-311.pyc"),
            "bytecode",
        )
        .unwrap();

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

        // Verify no __pycache__ directory or its contents are included at any level
        assert!(
            !relative_paths.iter().any(|p| p.contains("__pycache__")),
            "Bundle should not contain any __pycache__ directory or its contents"
        );

        // Verify expected files ARE included
        assert!(relative_paths.contains(&"main.py".to_string()));
        assert!(
            relative_paths
                .iter()
                .any(|p| p == "pkg/mod.py" || p == "pkg\\mod.py")
        );
    }

    #[test]
    fn test_prepare_bundle_excludes_pycache_even_when_included() {
        let temp_dir = tempdir().unwrap();
        let dir_path = temp_dir.path();

        fs::write(dir_path.join("main.py"), "print('hello')").unwrap();
        fs::create_dir(dir_path.join("__pycache__")).unwrap();
        fs::write(
            dir_path.join("__pycache__").join("main.cpython-311.pyc"),
            "bytecode",
        )
        .unwrap();

        // Try to explicitly include __pycache__ in the include patterns
        let include = Some(vec![
            "__pycache__".to_string(),
            "__pycache__/**".to_string(),
            "**/*.py".to_string(),
        ]);

        let result = prepare_bundle(dir_path, include, None).unwrap();

        let relative_paths: Vec<String> = result
            .iter()
            .map(|p| {
                p.strip_prefix(dir_path)
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        // Verify __pycache__ is STILL NOT included despite being in include patterns
        assert!(
            !relative_paths.iter().any(|p| p.contains("__pycache__")),
            "Bundle should not contain __pycache__ even when explicitly included"
        );

        // Verify main.py is included (matches **/*.py pattern)
        assert!(relative_paths.contains(&"main.py".to_string()));
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
