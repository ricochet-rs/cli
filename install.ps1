# Ricochet CLI installer for Windows PowerShell
# Usage: irm https://raw.githubusercontent.com/ricochet-rs/cli/main/install.ps1 | iex

$ErrorActionPreference = 'Stop'

# Configuration
$Version = if ($env:RICOCHET_VERSION) { $env:RICOCHET_VERSION } else { "0.1.0" }
$InstallDir = if ($env:RICOCHET_INSTALL_DIR) { $env:RICOCHET_INSTALL_DIR } else { "$HOME\bin" }
$GithubReleasesBase = "https://github.com/ricochet-rs/cli/releases/download/v$Version"

# Detect architecture
$Arch = $env:PROCESSOR_ARCHITECTURE
if ($Arch -eq "AMD64" -or $Arch -eq "x64") {
    $Tarball = "ricochet-$Version-windows-x86_64.exe.tar.gz"
    $BinaryName = "ricochet-$Version-windows-x86_64.exe"
} else {
    Write-Error "Unsupported Windows architecture: $Arch"
    exit 1
}

$Url = "$GithubReleasesBase/$Tarball"

Write-Host "Installing Ricochet CLI v$Version (Windows $Arch)..." -ForegroundColor Cyan

# Create install directory if it doesn't exist
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

# Create temporary directory
$TmpDir = New-TemporaryFile | ForEach-Object { Remove-Item $_; New-Item -ItemType Directory -Path $_ }

try {
    # Download
    Write-Host "Downloading from $Url..." -ForegroundColor Cyan
    $TarballPath = Join-Path $TmpDir $Tarball
    Invoke-WebRequest -Uri $Url -OutFile $TarballPath -UseBasicParsing

    # Extract (requires tar, available in Windows 10 1803+)
    Write-Host "Extracting..." -ForegroundColor Cyan
    tar -xzf $TarballPath -C $TmpDir

    # Install binary
    $FinalName = "ricochet.exe"
    $SourcePath = Join-Path $TmpDir $BinaryName
    $DestPath = Join-Path $InstallDir $FinalName
    
    Move-Item -Path $SourcePath -Destination $DestPath -Force

    Write-Host "`n✓ Ricochet CLI installed successfully!" -ForegroundColor Green
    Write-Host "Binary installed to: $DestPath" -ForegroundColor Gray
    Write-Host ""

    # Check if directory is in PATH
    $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $MachinePath = [Environment]::GetEnvironmentVariable("Path", "Machine")
    $FullPath = "$UserPath;$MachinePath"

    if ($FullPath -like "*$InstallDir*") {
        Write-Host "Run 'ricochet --help' to get started." -ForegroundColor Cyan
    } else {
        Write-Host "⚠️  Note: $InstallDir is not in your PATH." -ForegroundColor Yellow
        Write-Host ""
        Write-Host "To add it to your PATH permanently, run:" -ForegroundColor Gray
        Write-Host "  `$env:Path += `";$InstallDir`"; [Environment]::SetEnvironmentVariable('Path', `$env:Path, 'User')" -ForegroundColor White
        Write-Host ""
        Write-Host "Or for current session only:" -ForegroundColor Gray
        Write-Host "  `$env:Path += `";$InstallDir`"" -ForegroundColor White
        Write-Host ""
        Write-Host "For now, you can run: $DestPath --help" -ForegroundColor Gray
    }
} finally {
    # Cleanup
    Remove-Item -Path $TmpDir -Recurse -Force -ErrorAction SilentlyContinue
}
