# cli

## Installation

Linux, macOS, Windows (Git Bash/WSL):

```bash
curl -fsSL https://raw.githubusercontent.com/ricochet-rs/cli/main/install.sh | sh
```

Windows (PowerShell/CMD):

```powershell
curl.exe -fsSL https://raw.githubusercontent.com/ricochet-rs/cli/main/install.ps1 -o install.ps1; .\install.ps1; Remove-Item install.ps1
```

> [!WARNING]
> The default install dir is `$HOME/bin` for your current user, which is not in `$PATH` by default.
> You need to add this directory to your `$PATH` environment variable or otherwise always use the full path to the binary.

Or using Git Bash/WSL:

```bash
curl -fsSL https://raw.githubusercontent.com/ricochet-rs/cli/main/install.sh | sh
```

> [!TIP]
> If you're on macOS, you might want to use our [homebrew tap](https://github.com/ricochet-rs/homebrew-tap) instead.

The installer will automatically detect your OS and architecture and download the appropriate binary from the release artifacts.

> [!NOTE]
> We will add support for [scoop](https://scoop.sh/) and [chocolatey](https://chocolatey.org/install) in the future.

### Customization

You can customize the installation with environment variables:

Bash/sh:

```bash
# Install a specific version
RICOCHET_VERSION=0.2.0 curl -fsSL https://raw.githubusercontent.com/ricochet-rs/cli/main/install.sh | sh

# Install to a custom directory
RICOCHET_INSTALL_DIR=$HOME/.local/bin curl -fsSL https://raw.githubusercontent.com/ricochet-rs/cli/main/install.sh | sh
```

PowerShell:

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
