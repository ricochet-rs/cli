# Ricochet CLI installer for Windows PowerShell
# Usage: curl.exe -fsSL https://raw.githubusercontent.com/ricochet-rs/cli/main/install.ps1 -o install.ps1; .\install.ps1; Remove-Item install.ps1

$ErrorActionPreference = 'Stop'

function Test-PathContainsDirectory {
    param([string]$PathValue, [string]$Directory)

    $Target = [Environment]::ExpandEnvironmentVariables($Directory).TrimEnd('\')
    foreach ($Entry in $PathValue -split ';') {
        $Expanded = [Environment]::ExpandEnvironmentVariables($Entry).Trim().TrimEnd('\')
        if ($Expanded -and $Expanded -ieq $Target) {
            return $true
        }
    }
    return $false
}

function Send-EnvironmentChangeBroadcast {
    # Explorer caches the environment and hands the stale copy to every process it
    # starts, so without WM_SETTINGCHANGE a new terminal keeps the old Path.
    if (-not ('Ricochet.NativeMethods' -as [type])) {
        Add-Type -Namespace 'Ricochet' -Name 'NativeMethods' -MemberDefinition @'
[DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Auto)]
public static extern IntPtr SendMessageTimeout(IntPtr hWnd, uint Msg, UIntPtr wParam, string lParam, uint fuFlags, uint uTimeout, out UIntPtr lpdwResult);
'@
    }

    $Unused = [UIntPtr]::Zero
    [void][Ricochet.NativeMethods]::SendMessageTimeout([IntPtr]0xffff, 0x1A, [UIntPtr]::Zero, 'Environment', 0x0002, 5000, [ref]$Unused)
}

function Join-PathEntry {
    param([string]$PathValue, [string]$Directory)

    # An empty leading or trailing entry makes Windows search the current directory,
    # so append to the trimmed value rather than concatenating a bare separator.
    $Trimmed = $PathValue.Trim().Trim(';')
    if ([string]::IsNullOrEmpty($Trimmed)) {
        return $Directory
    }
    return "$Trimmed;$Directory"
}

function Add-DirectoryToUserPath {
    param([string]$Directory)

    # HKCU is edited directly because [Environment]::SetEnvironmentVariable rewrites
    # Path as REG_SZ, which freezes entries such as %USERPROFILE%\bin at today's value.
    $Key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment', $true)
    try {
        $Existing = $Key.GetValue('Path', $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
        if ([string]::IsNullOrEmpty($Existing)) {
            $Kind = [Microsoft.Win32.RegistryValueKind]::ExpandString
        } else {
            $Kind = $Key.GetValueKind('Path')
        }
        $Key.SetValue('Path', (Join-PathEntry $Existing $Directory), $Kind)
    } finally {
        if ($Key) { $Key.Dispose() }
    }
}

# Configuration
$Version = if ($env:RICOCHET_VERSION) { $env:RICOCHET_VERSION } else { "1.1.0" }
$InstallDir = if ($env:RICOCHET_INSTALL_DIR) { $env:RICOCHET_INSTALL_DIR } else { "$HOME\bin" }
$GithubReleasesBase = "https://github.com/ricochet-rs/cli/releases/download/v$Version"

# Detect architecture
$Arch = $env:PROCESSOR_ARCHITECTURE
if ($Arch -eq "AMD64" -or $Arch -eq "x64") {
    $Tarball = "ricochet-$Version-windows-x86_64.exe.tar.gz"
    $BinaryName = "ricochet-$Version-windows-x86_64.exe"
} else {
    Write-Host "Unsupported Windows architecture: $Arch" -ForegroundColor Red
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

    # Windows only executes a file with an executable extension, so keep .exe.
    $FinalName = "ricochet.exe"
    $SourcePath = Join-Path $TmpDir $BinaryName
    $DestPath = Join-Path $InstallDir $FinalName

    Move-Item -Path $SourcePath -Destination $DestPath -Force

    Write-Host ""
    Write-Host "Ricochet CLI installed successfully!" -ForegroundColor Green
    Write-Host "Binary installed to: $DestPath" -ForegroundColor Gray
    Write-Host ""

    # Put the install directory on PATH so 'ricochet' resolves without a full path.
    $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $MachinePath = [Environment]::GetEnvironmentVariable("Path", "Machine")

    if (Test-PathContainsDirectory "$UserPath;$MachinePath" $InstallDir) {
        Write-Host "Run 'ricochet --help' to get started." -ForegroundColor Cyan
    } else {
        try {
            Add-DirectoryToUserPath $InstallDir
            $env:Path = "$env:Path;$InstallDir"
            Write-Host "Added $InstallDir to your user PATH." -ForegroundColor Green

            try {
                Send-EnvironmentChangeBroadcast
                Write-Host "Open a new terminal, then run 'ricochet --help' to get started." -ForegroundColor Cyan
            } catch {
                Write-Host "Sign out and back in for other terminals to pick it up." -ForegroundColor Yellow
                Write-Host "Run 'ricochet --help' to get started in this terminal." -ForegroundColor Cyan
            }
        } catch {
            Write-Host "Warning: could not add $InstallDir to your PATH: $_" -ForegroundColor Yellow
            Write-Host ""
            Write-Host "To add it to your PATH for current session:" -ForegroundColor Gray
            Write-Host "  `$env:Path += `";$InstallDir`"" -ForegroundColor White
            Write-Host ""
            Write-Host "For now, you can run: $DestPath --help" -ForegroundColor Gray
        }
    }
} catch {
    Write-Host "Error: $_" -ForegroundColor Red
    exit 1
} finally {
    # Cleanup
    if (Test-Path $TmpDir) {
        Remove-Item -Path $TmpDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}
