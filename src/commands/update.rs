use crate::update;
use anyhow::{Context, Result};
use colored::Colorize;
use flate2::read::GzDecoder;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::Read;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Determine the download URL for the current platform.
fn download_url(version: &str) -> Result<String> {
    #[cfg(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
    ))]
    let base = format!(
        "https://github.com/ricochet-rs/cli/releases/download/v{}",
        version
    );

    #[cfg(target_os = "macos")]
    let base = format!(
        "https://hel1.your-objectstorage.com/ricochet-cli/v{}",
        version
    );

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return Ok(format!("{}/ricochet-{}-linux-x86_64.tar.gz", base, version));

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    return Ok(format!(
        "{}/ricochet-{}-linux-aarch64.tar.gz",
        base, version
    ));

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return Ok(format!(
        "{}/ricochet-{}-windows-x86_64.exe.tar.gz",
        base, version
    ));

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return Ok(format!("{}/ricochet-{}-darwin-arm64.tar.gz", base, version));

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return Ok(format!(
        "{}/ricochet-{}-darwin-x86_64.tar.gz",
        base, version
    ));

    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
    )))]
    anyhow::bail!(
        "self-update is not supported on this platform.\n  Install manually: https://github.com/ricochet-rs/cli/releases"
    );
}

/// The binary name inside the tarball for the current platform.
/// macOS bottles have a different structure: ricochet/{version}/bin/ricochet
fn binary_name_in_tarball(version: &str) -> Result<String> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return Ok(format!("ricochet-{}-linux-x86_64", version));

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    return Ok(format!("ricochet-{}-linux-aarch64", version));

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return Ok(format!("ricochet-{}-windows-x86_64.exe", version));

    // macOS bottles: the binary is at ricochet/{version}/bin/ricochet inside the tarball
    #[cfg(target_os = "macos")]
    return Ok(format!("ricochet/{}/bin/ricochet", version));

    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
        target_os = "macos",
    )))]
    {
        let _ = version;
        anyhow::bail!("self-update is not supported on this platform");
    }
}

pub async fn self_update(force: bool) -> Result<()> {
    println!("Checking for updates...");

    let latest = update::fetch_latest_version()
        .await
        .context("Failed to fetch latest version from GitHub")?;

    let latest_cache = update::UpdateCache::for_version(latest.clone());

    if !latest_cache.is_update_available() && !force {
        println!(
            "{} Already on the latest version: {}",
            "✓".green().bold(),
            CURRENT_VERSION.bright_cyan()
        );
        return Ok(());
    }

    if !latest_cache.is_update_available() {
        println!(
            "Reinstalling current version {} (--force)",
            latest.bright_cyan()
        );
    } else {
        println!(
            "Updating {} -> {}",
            CURRENT_VERSION.dimmed(),
            latest.green().bold()
        );
    }

    let url = download_url(&latest)?;

    let client = reqwest::Client::builder()
        .user_agent(concat!("ricochet-cli/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    spinner.set_message(format!("Downloading v{}...", latest));
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));

    let tarball_bytes = client
        .get(&url)
        .send()
        .await
        .context("Failed to download release tarball")?
        .error_for_status()
        .context("Download failed (server returned error)")?
        .bytes()
        .await
        .context("Failed to read download response")?;

    spinner.finish_and_clear();
    println!("{} Downloaded ({} bytes)", "✓".green(), tarball_bytes.len());

    let binary_path = binary_name_in_tarball(&latest)?;
    let extracted_bytes = extract_binary_from_tarball(&tarball_bytes, &binary_path)
        .with_context(|| format!("Failed to extract '{}' from tarball", binary_path))?;

    // Write extracted binary to a temp file, then use self_replace to atomically swap it in.
    // self_replace handles platform quirks like Windows locking the running executable.
    let tmp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
    let tmp_path = tmp_dir.path().join("ricochet-new");
    std::fs::write(&tmp_path, &extracted_bytes)
        .context("Failed to write new binary to temp file")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o755))
            .context("Failed to set executable permissions")?;
    }

    self_replace::self_replace(&tmp_path)
        .context("Failed to replace the ricochet binary. You may need elevated permissions.")?;

    // Update the cache: reset failure counter and record the new version
    let _ = update::UpdateCache::for_version(latest.clone()).save();

    // Re-enable update checks if they were auto-disabled due to previous failures
    if let Ok(mut config) = crate::config::Config::load()
        && config.skip_update_check == Some(true)
    {
        config.re_enable_update_checks();
    }

    println!(
        "\n{} Successfully updated to v{}",
        "✓".green().bold(),
        latest.bright_cyan()
    );
    println!(
        "Release notes: {}",
        update::release_notes_url(&latest).bright_cyan()
    );

    Ok(())
}

fn extract_binary_from_tarball(tarball: &[u8], binary_path: &str) -> Result<Vec<u8>> {
    use std::io::Cursor;

    let cursor = Cursor::new(tarball);
    let gz = GzDecoder::new(cursor);
    let mut archive = tar::Archive::new(gz);

    for entry in archive.entries().context("Failed to read tar entries")? {
        let mut entry = entry.context("Failed to read tar entry")?;
        let path = entry.path().context("Failed to read entry path")?;
        let path_str = path.to_string_lossy();

        // Match against full path (for macOS bottles: ricochet/0.4.0/bin/ricochet)
        // or just the filename (for Linux/Windows flat tarballs)
        let matches = path_str == binary_path
            || path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n == binary_path)
                .unwrap_or(false);

        if matches {
            let mut bytes = Vec::new();
            entry
                .read_to_end(&mut bytes)
                .context("Failed to read binary from tar")?;
            return Ok(bytes);
        }
    }

    anyhow::bail!("Binary '{}' not found in tarball", binary_path)
}
