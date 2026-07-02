# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

This project uses Just as the task runner. Common commands:

- `just` - List all available commands
- `just check` - Run full check including format, clippy, and build
- `just lint` - Run clippy linter
- `just lint-fix` - Run clippy with automatic fixes
- `just fmt` - Format code
- `just cli-build` - Build the CLI for current platform

Always run `just lint` after finishing code changes.

### Build Targets

The project supports cross-platform builds:

- `just cli-build-all` - Build for all platforms
- `just cli-cross-build-all` - Build using cross for better compatibility
- Individual platform builds: `cli-build-linux-x64`, `cli-build-macos-arm64`, etc.

### Standard Cargo Commands

- `cargo build --release --bin ricochet` - Build the CLI binary
- `cargo check --all-features --workspace` - Check code without building
- `cargo clippy --all-targets --all-features -- -D warnings` - Run linter

## Architecture

This is a Rust CLI application called `ricochet` that provides a client interface to the Ricochet platform for deploying and managing content.

### Core Components

- **Main Binary**: `src/main.rs` - CLI entry point using clap for argument parsing
- **Client**: `src/client.rs` - HTTP client (`RicochetClient`) that handles all API communication with the Ricochet server
- **Config**: `src/config.rs` - Configuration management using TOML format, stored in user's config directory
- **Commands**: `src/commands/` - Individual command implementations (auth, deploy, list, delete, servers, config)
- **Utils**: `src/utils.rs` - Utility functions including bundle creation

### Command Structure

The CLI follows a subcommand pattern with these main operations:

- `login/logout` - Authentication management (supports `--server` flag)
- `deploy` - Upload content to a ricochet server (supports `--server` flag)
- `list` - List deployed content items with filtering
- `delete` - Remove content items
- `servers` - Manage multiple server configurations (list, add, remove, set-default)
- `config` - Show configuration

### External Dependencies

The project depends on two private Ricochet libraries:

- `ricochet-core` - Core platform functionality
- `ricochet-auth` - Authentication utilities

These are currently pulled from Git with SSH access. For local development, they can be overridden with local paths in Cargo.toml.

### Configuration

Configuration is stored in `~/.config/ricochet/config.toml` (all platforms) and supports:

- Multiple named server configurations with URLs and API keys
- Default server selection
- Environment variable overrides: `RICOCHET_SERVER` and `RICOCHET_API_KEY`
- Default output format

Legacy single-server configs are automatically migrated to multi-server format.

### Output Formats

The CLI supports multiple output formats: `table` (default), `json`, and `yaml`.

### CI/CD

CI pipelines are in `.crow/` (Crow/Woodpecker CI):

- `binaries.yaml` - Linux/Windows builds, S3 upload, GitHub release, version bump, Homebrew tap trigger (runs on tag)
- `binaries-mac.yaml` - macOS builds and S3 upload (runs on tag)
- `changelog.yaml` - Updates upcoming changelog issue on push to main
- `lint.yaml` / `test.yaml` - PR checks

On a tagged release, the `binaries.yaml` pipeline:

1. Updates `Cargo.toml` version from the tag, regenerates `Cargo.lock`
2. Builds binaries, uploads to S3, creates GitHub release
3. Bumps version in `install.sh` and `install.ps1`, commits and pushes to main
4. Triggers the `homebrew-tap` repo's `update-formula` workflow

### Development Notes

- Git hooks use **prek** (`prek.toml`), not pre-commit; hooks should mirror the checks in `.crow/lint.yaml`
- Run `cargo check --all-targets` before committing so compile errors are caught locally, not in CI
- Tests are in `tests/` directory and inline in source files (`#[cfg(test)]`)
- Run tests with `cargo test --all-features` (use `--test-threads=1` for env var tests)
- The project uses tokio for async runtime
- HTTP client uses reqwest with 5-minute timeout
- File uploads support both individual files and directory bundling (creates tar.gz)
- Uses ULID for unique identifiers
