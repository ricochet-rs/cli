# cli

## Installation

### Quick Install

```bash
curl -fsSL https://raw.githubusercontent.com/ricochet/cli/main/install.sh | sh
```

The installer will automatically detect your OS and architecture and download the appropriate binary from the release artifacts.

>[!TIP] If you're on macOS, you might want to use our [homebrew tap](https://github.com/ricochet-rs/homebrew-tap) instead.

#### Customization

You can customize the installation with environment variables:

```bash
# Install a specific version
RICOCHET_VERSION=0.2.0 curl -fsSL https://raw.githubusercontent.com/ricochet/cli/main/install.sh | sh

# Install to a custom directory
RICOCHET_INSTALL_DIR=$HOME/.local/bin curl -fsSL https://raw.githubusercontent.com/ricochet/cli/main/install.sh | sh
```

## Commands

See the full documentation at [docs/cli-commands.md](docs/cli-commands.md).

## Development

### Documentation

The CLI documentation is automatically generated from the command-line help.

```bash
just docs
```
