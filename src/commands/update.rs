use crate::update;
use anyhow::{Context, Result};
use colored::Colorize;
use flate2::read::GzDecoder;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::Read;
use std::path::Path;

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
        "https://s3.eu-central-003.backblazeb2.com/ricochet-cli/v{}",
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
fn binary_name_in_tarball(version: &str) -> Result<String> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return Ok(format!("ricochet-{}-linux-x86_64", version));

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    return Ok(format!("ricochet-{}-linux-aarch64", version));

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return Ok(format!("ricochet-{}-windows-x86_64.exe", version));

    // macOS bottles contain a flat "ricochet" binary (tarball created from inside the bin dir)
    #[cfg(target_os = "macos")]
    {
        let _ = version;
        Ok("ricochet".to_string())
    }

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

pub fn print_version() {
    let version = env!("CARGO_PKG_VERSION");
    let git_hash = env!("GIT_HASH");
    let has_tag = env!("HAS_GIT_TAG");
    let build_date = env!("BUILD_DATE");

    if has_tag == "true" || git_hash.is_empty() {
        println!("{}", version);
    } else {
        println!("{}-{} ({})", version, git_hash, build_date);
    }
}

pub async fn self_update(force: bool, dry_run: bool) -> Result<()> {
    let package_manager = update::PackageManager::detect_current();
    if let Some(package_manager) = package_manager
        && !dry_run
    {
        anyhow::bail!(
            "ricochet was installed with {}, so it cannot update itself.\n  Update with: {}",
            package_manager.name(),
            package_manager.upgrade_hint()
        );
    }

    println!("Checking for updates...");

    let latest = update::fetch_latest_version()
        .await
        .context("Failed to fetch latest version from GitHub")?;

    let latest_cache = update::UpdateCache::for_version(latest.clone());

    if dry_run {
        println!("  Current version: {}", CURRENT_VERSION.bright_cyan());
        println!("  Latest version:  {}", latest.bright_cyan());

        if latest_cache.is_update_available() {
            println!(
                "\n{} Update available: {} -> {}",
                "→".green().bold(),
                CURRENT_VERSION.dimmed(),
                latest.green().bold()
            );
            println!(
                "  Release notes: {}",
                update::release_notes_url(&latest).bright_cyan()
            );
            let update_command = package_manager
                .map_or("ricochet self update", update::PackageManager::upgrade_hint);
            println!("\nRun {} to update.", update_command.bright_cyan());
        } else {
            println!("\n{} Already on the latest version.", "✓".green().bold());
        }

        return Ok(());
    }

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

    replace_current_executable(&tmp_path)?;

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

/// Swap the running binary for `new_executable`, using `self_replace` for Windows's locked executable.
#[cfg(windows)]
fn replace_current_executable(new_executable: &Path) -> Result<()> {
    self_replace::self_replace(new_executable).context(
        "Failed to replace the ricochet binary. Run the terminal as administrator and try again.",
    )
}

/// Swap the running binary for `new_executable`, asking for `sudo` when its directory is not writable.
///
/// The path is resolved first because macOS reports the unresolved symlink as the executable,
/// and a Homebrew-style relative link target would otherwise be read against the working directory.
#[cfg(unix)]
fn replace_current_executable(new_executable: &Path) -> Result<()> {
    let exe = std::env::current_exe()
        .and_then(|exe| exe.canonicalize())
        .context("Failed to locate the running ricochet binary")?;

    match replace_executable(new_executable, &exe) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
            replace_executable_with_sudo(new_executable, &exe)
        }
        Err(err) => Err(err)
            .with_context(|| format!("Failed to replace the ricochet binary at {}", exe.display())),
    }
}

/// Atomically rename a copy of `new_executable` over `exe` within the same directory.
#[cfg(unix)]
fn replace_executable(new_executable: &Path, exe: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let dir = exe.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "executable has no parent directory",
        )
    })?;
    let staged = tempfile::Builder::new()
        .prefix(".ricochet.__update__")
        .tempfile_in(dir)?;
    std::fs::copy(new_executable, staged.path())?;
    std::fs::set_permissions(staged.path(), std::fs::Permissions::from_mode(0o755))?;
    staged.persist(exe).map_err(|err| err.error)?;
    Ok(())
}

#[cfg(unix)]
fn replace_executable_with_sudo(new_executable: &Path, exe: &Path) -> Result<()> {
    use std::io::IsTerminal;

    let manual_hint = format!(
        "Run {} or reinstall into a directory you can write to.",
        "sudo ricochet self update".bright_cyan()
    );
    if !std::io::stdin().is_terminal() {
        anyhow::bail!(
            "Writing to {} requires elevated permissions.\n  {manual_hint}",
            exe.display()
        );
    }

    println!(
        "Writing to {} requires elevated permissions, running sudo...",
        exe.display()
    );
    // `install` unlinks the target before copying, so the running binary's inode stays intact.
    let status = std::process::Command::new("sudo")
        .arg("install")
        .args(["-m", "755"])
        .arg(new_executable)
        .arg(exe)
        .status()
        .context("Failed to run sudo")?;
    if !status.success() {
        anyhow::bail!(
            "sudo failed to replace the ricochet binary at {}.\n  {manual_hint}",
            exe.display()
        );
    }
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

        // Match against full path or just the filename
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

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    /// Create a tar.gz in memory with the given entries: (path, content).
    fn make_tarball(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut tar_bytes = Vec::new();
        {
            let mut builder = tar::Builder::new(&mut tar_bytes);
            for (path, content) in entries {
                let mut header = tar::Header::new_gnu();
                header.set_size(content.len() as u64);
                header.set_mode(0o755);
                header.set_cksum();
                builder.append_data(&mut header, *path, *content).unwrap();
            }
            builder.finish().unwrap();
        }

        let mut gz_bytes = Vec::new();
        let mut encoder = GzEncoder::new(&mut gz_bytes, Compression::fast());
        encoder.write_all(&tar_bytes).unwrap();
        encoder.finish().unwrap();
        gz_bytes
    }

    #[test]
    fn extract_flat_binary_by_exact_name() {
        let content = b"fake-binary-content";
        let tarball = make_tarball(&[("ricochet-0.5.0-linux-x86_64", content)]);

        let result = extract_binary_from_tarball(&tarball, "ricochet-0.5.0-linux-x86_64").unwrap();
        assert_eq!(result, content);
    }

    #[test]
    fn extract_flat_binary_by_filename_fallback() {
        // The tarball has "ricochet" and we search for "ricochet" — macOS case
        let content = b"macos-binary";
        let tarball = make_tarball(&[("ricochet", content)]);

        let result = extract_binary_from_tarball(&tarball, "ricochet").unwrap();
        assert_eq!(result, content);
    }

    #[test]
    fn extract_returns_error_when_not_found() {
        let tarball = make_tarball(&[("some-other-file", b"data")]);

        let result = extract_binary_from_tarball(&tarball, "ricochet");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("not found in tarball")
        );
    }

    #[test]
    fn extract_ignores_non_matching_entries() {
        let content = b"the-right-one";
        let tarball = make_tarball(&[
            ("README.md", b"readme"),
            ("ricochet-0.5.0-linux-x86_64", content),
            ("other-file", b"nope"),
        ]);

        let result = extract_binary_from_tarball(&tarball, "ricochet-0.5.0-linux-x86_64").unwrap();
        assert_eq!(result, content);
    }

    #[cfg(unix)]
    #[test]
    fn replace_executable_swaps_file_contents() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("ricochet");
        let new_executable = dir.path().join("ricochet-new");
        std::fs::write(&exe, b"old").unwrap();
        std::fs::write(&new_executable, b"new").unwrap();

        replace_executable(&new_executable, &exe).unwrap();

        assert_eq!(std::fs::read(&exe).unwrap(), b"new");
        let mode = std::fs::metadata(&exe).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o755);
        let leftovers: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
        assert_eq!(leftovers.len(), 2);
    }

    #[cfg(unix)]
    #[test]
    fn replace_executable_reports_permission_denied_for_readonly_dir() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("ricochet");
        std::fs::write(&exe, b"old").unwrap();
        let new_dir = tempfile::tempdir().unwrap();
        let new_executable = new_dir.path().join("ricochet-new");
        std::fs::write(&new_executable, b"new").unwrap();
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o555)).unwrap();

        let result = replace_executable(&new_executable, &exe);
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();

        // Root ignores directory permissions, so only assert when the write was refused.
        if let Err(err) = result {
            assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
        }
    }

    #[test]
    fn binary_name_in_tarball_current_platform() {
        let name = binary_name_in_tarball("0.5.0").unwrap();
        // On macOS it should be just "ricochet", on Linux it should include the version
        #[cfg(target_os = "macos")]
        assert_eq!(name, "ricochet");
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        assert_eq!(name, "ricochet-0.5.0-linux-x86_64");
        #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
        assert_eq!(name, "ricochet-0.5.0-linux-aarch64");
        // Just verify it doesn't error on whatever platform we're on
        assert!(!name.is_empty());
    }
}
