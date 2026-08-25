---
name: ricochet-cli
description: Use when deploying or operating Ricochet content with the `ricochet` CLI, including authoring a `_ricochet.toml`, connecting to servers, managing environment variables, invoking or scheduling tasks, and diagnosing CLI errors.
---

# Use the Ricochet CLI

`ricochet` deploys content to a Ricochet server and manages that content afterwards.
Read [references/ricochet-toml.md](references/ricochet-toml.md) before writing or editing a `_ricochet.toml`.
Run `ricochet <COMMAND> --help` for exhaustive flags, or read `docs/cli-commands.md` in the `ricochet-rs/cli` repository.

## Work non-interactively

`ricochet init` refuses to run when stdin is not a terminal, when `CI` is set, when `CARGO_MANIFEST_DIR` is set, or when `RICOCHET_NON_INTERACTIVE` is set.
Write `_ricochet.toml` by hand instead of calling `ricochet init` from an agent session or a CI job.
Under the same conditions `ricochet deploy` fails with `No _ricochet.toml found` rather than offering to create one.

Pass `-F json` to any command whose output must be parsed.
Pass `--debug` to surface the underlying request and response when a command fails.

## Connect to a server

Servers are named profiles in `~/.config/ricochet/config.toml` on every platform.
Add one, then make it the default:

```sh
ricochet server add production https://ricochet.example.com --default
ricochet server list
ricochet server set-default production
```

The URL must carry an `http://` or `https://` scheme.
`ricochet login` opens a browser callback flow, falls back to pasting a key when no display server is present, and accepts a key directly with `-k`:

```sh
ricochet login -S production
ricochet login -S https://ricochet.example.com -k rico_...
```

Keys minted by `ricochet login` expire after 8 hours, so a long-lived automation key must be created in the web UI and supplied through `RICOCHET_API_KEY`.
`ricochet config` prints the resolved configuration, and `ricochet config -A` includes the stored keys.

Server resolution runs in this order:

1. `RICOCHET_SERVER`, which is read before anything else and therefore overrides `-S`.
2. The `-S/--server` value, matched first against profile names and then against configured URLs.
3. The `default_server` profile.

`RICOCHET_API_KEY` replaces the stored key for whichever server was resolved.
An unknown `-S` value fails with `Server '<name>' not found` and lists the configured profiles.

## Describe the content item

Every deployable directory holds a `_ricochet.toml` next to its package lockfile.
A minimal task looks like this:

```toml
[content]
name = "weekly-report"
entrypoint = "report.qmd"
access_type = "private"
content_type = "quarto-r"

[language]
name = "r"
packages = "renv.lock"

[schedule]
cron = "0 9 * * 1-5"
```

`content.id` is absent on a brand new item.
The first successful `ricochet deploy` writes the assigned ULID back into the file, and every later deploy of that directory updates the same item.
Most `app` and `task` subcommands read that ID from the local file when the `ID` argument is omitted, so run them from the content directory or point `-p` at the file.

The lockfile named by `language.packages` must exist before deploying: `renv.lock` for R, `uv.lock` for Python, and `Manifest.toml` for Julia.
Python items additionally need a `.python-version`.
For both files the CLI searches parent directories, which is how uv workspaces keep a single lockfile at the workspace root.

## Deploy

```sh
ricochet deploy
ricochet deploy ./reports -S staging
ricochet deploy -e DATABASE_URL -e LOG_LEVEL=debug
```

The path argument must be a directory containing `_ricochet.toml`, not the TOML file itself.
Everything under that directory is bundled except `.venv`, `.renv`, and `__pycache__`, which are dropped even when an `include` pattern names them.
Narrow the bundle with `content.include` and `content.exclude` globs.

`-e KEY=VALUE` sets a variable directly, while `-e KEY` alone resolves the value from `.env`, then `.Renviron`, then the calling environment.
Values are encrypted with the server's public key before they leave the machine, and only the keys named on the command line are sent.
`-e` applies to a content item's first deployment only; use `ricochet app env-vars` or `ricochet task env-vars` after that.

Git-backed items skip local bundling entirely:

```sh
ricochet user credentials
ricochet deploy --git https://github.com/acme/reports --branch main --path dashboards --credential <CREDENTIAL_ID>
```

## Operate apps

```sh
ricochet app list -a --sort=-updated
ricochet app instances
ricochet app stop
ricochet app stop <ID> <INSTANCE_ID>
ricochet app toml <ID>
ricochet app deployment list <ID> --fields all
ricochet app deployment get <DEPLOYMENT_ID>
```

`ricochet app stop` with no instance argument stops every instance of the item.
`ricochet app toml` fetches the deployed `_ricochet.toml`, which is the fastest way to see what the server actually believes about an item.

## Operate tasks

Tasks are the content types that can be invoked rather than served: `r`, `rmd`, `julia`, `python`, `quarto-r`, `quarto-jl`, and `quarto-py`.

```sh
ricochet task list
ricochet task invoke <ID>
ricochet task schedule <ID> "0 9 * * 1-5"
```

`ricochet task invoke` starts a run and returns the invocation record immediately, so its reported status is usually `pending` or `running` rather than a final result.
The CLI has no command that polls an invocation, so follow the run in the web UI.
`ricochet task schedule` validates the cron expression locally, then prints the parsed description and the next run time in UTC.
A schedule can also be declared in `[schedule]` and applied with `ricochet task settings update`.

## Change settings after the first deployment

`_ricochet.toml` edits do not reach the server until they are applied explicitly:

```sh
ricochet app settings
ricochet app settings update
ricochet task settings update -f
```

The bare `settings` command prints the diff between the local file and the deployed item.
`update` applies it, prompting unless `-f` is given.

## Manage environment variables

```sh
ricochet app env-vars get
ricochet app env-vars set API_TOKEN LOG_LEVEL=debug
ricochet app env-vars delete API_TOKEN
ricochet app env-vars replace API_TOKEN=abc
```

`get` returns names only, never values.
`set` upserts and leaves unlisted variables untouched, while `replace` deletes every variable that is not listed, including all of them when no argument is given.
The bare `KEY` form resolves from `.env`, `.Renviron`, or the environment, exactly as it does for `ricochet deploy -e`.
The same four subcommands exist under `ricochet task`.

## Diagnose common errors

| Message                                                                      | Cause and fix                                                                                                                                                                                    |
| ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `No API key configured`                                                      | The resolved server profile has no key. Run `ricochet login -S <profile>`, or set `RICOCHET_API_KEY`.                                                                                            |
| `Credentials are invalid or expired for server <url>`                        | The stored key is missing, wrong, or past its 8 hour lifetime. Run `ricochet login -S <url>`.                                                                                                    |
| `Cannot run init in non-interactive mode`                                    | `ricochet init` needs a TTY. Write `_ricochet.toml` by hand using the schema reference.                                                                                                          |
| `No _ricochet.toml found in <dir>`                                           | Non-interactive deploy without a config file. Create the file, or point the path argument at the directory that holds it.                                                                        |
| `Path must be a directory containing _ricochet.toml`                         | The path argument pointed at a file. Pass the containing directory.                                                                                                                              |
| ``Required package file `renv.lock` not found``                              | Run `renv::snapshot()` in R, `uv init` for `uv.lock`, or `Pkg.instantiate()` in Julia.                                                                                                           |
| ``Please create a `.python-version` via `uv python pin```                    | Python items need a pinned interpreter version beside `uv.lock` or at the workspace root.                                                                                                        |
| `403` while redeploying an existing ID                                       | The ID does not exist on this server, the key lacks permission, or the item belongs to another server. Verify with `ricochet app list`, check `-S`, or delete `content.id` to create a new item. |
| `Environment variables can only be set on a content item's first deployment` | Redeploy without `-e`, then use `ricochet app env-vars set`.                                                                                                                                     |
| `Server '<name>' not found`                                                  | The profile is not configured. Run `ricochet server list`, then `ricochet server add`.                                                                                                           |
| `Server URL must include the scheme prefix`                                  | Prefix the URL with `http://` or `https://`.                                                                                                                                                     |
| `Local _ricochet.toml has no item ID`                                        | The item was never deployed. Run `ricochet deploy` once, or pass the ID explicitly.                                                                                                              |
| `Package file required`                                                      | A Python item declared a non-uv lockfile. Set `packages = "uv.lock"`.                                                                                                                            |
| `Chosen content type cannot be served`                                       | A task content type declared `[serve]`. Remove the table.                                                                                                                                        |
| `Chosen content type cannot be scheduled. It must be served.`                | An app content type declared `[schedule]`. Remove the table.                                                                                                                                     |
| `Chosen content type cannot be served as a static site`                      | An app content type declared `[static]`. Remove the table.                                                                                                                                       |
| `Invalid entrypoint`                                                         | The entrypoint extension does not match the content type. See the validation rules in the schema reference.                                                                                      |

## Keep the CLI current

```sh
ricochet self update --dry-run
ricochet self update
```

Automatic update checks are suppressed when `CI` or `RICOCHET_NO_UPDATE_CHECK` is set, when `skip_update_check` is true in the config, and for Homebrew installations.
