# cli

## Installation

### Quick Install

Linux, macOS, Windows (Git Bash/WSL):

```bash
curl -fsSL https://raw.githubusercontent.com/ricochet-rs/cli/main/install.sh | sh
```

Windows (PowerShell/CMD):

```powershell
curl.exe -fsSL https://raw.githubusercontent.com/ricochet-rs/cli/main/install.ps1 -o install.ps1; .\install.ps1; Remove-Item install.ps1
```

Or using Git Bash/WSL:
```bash
curl -fsSL https://raw.githubusercontent.com/ricochet-rs/cli/main/install.sh | sh
```

Windows (Manual):

If you cannot execute PowerShell scripts, download and install manually:

```cmd
set VERSION=0.1.0
curl.exe -fsSL -O https://github.com/ricochet-rs/cli/releases/download/v%VERSION%/ricochet-%VERSION%-windows-x86_64.exe.tar.gz
tar -xzf ricochet-%VERSION%-windows-x86_64.exe.tar.gz
move ricochet-%VERSION%-windows-x86_64.exe "%USERPROFILE%\bin\ricochet.exe"
del ricochet-%VERSION%-windows-x86_64.exe.tar.gz
```

The installer will automatically detect your OS and architecture and download the appropriate binary from the release artifacts.

> [!TIP]
> If you're on macOS, you might want to use our [homebrew tap](https://github.com/ricochet-rs/homebrew-tap) instead.

#### Customization

You can customize the installation with environment variables:

**Bash/sh:**
```bash
# Install a specific version
RICOCHET_VERSION=0.2.0 curl -fsSL https://raw.githubusercontent.com/ricochet-rs/cli/main/install.sh | sh

# Install to a custom directory
RICOCHET_INSTALL_DIR=$HOME/.local/bin curl -fsSL https://raw.githubusercontent.com/ricochet-rs/cli/main/install.sh | sh
```

**PowerShell:**
```powershell
# Install a specific version
$env:RICOCHET_VERSION="0.2.0"; curl.exe -fsSL https://raw.githubusercontent.com/ricochet-rs/cli/main/install.ps1 -o install.ps1; .\install.ps1; Remove-Item install.ps1

# Install to a custom directory
$env:RICOCHET_INSTALL_DIR="$HOME\.local\bin"; curl.exe -fsSL https://raw.githubusercontent.com/ricochet-rs/cli/main/install.ps1 -o install.ps1; .\install.ps1; Remove-Item install.ps1
```

## Commands

See the full documentation at [docs/cli-commands.md](docs/cli-commands.md).

## Development

### Documentation

The CLI documentation is automatically generated from the command-line help.

```bash
just docs
```
