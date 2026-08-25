# `_ricochet.toml` schema

A `_ricochet.toml` describes one content item.
It lives in the directory that is deployed, beside the package lockfile.
The schema below matches `ricochet_core::content::ContentItem` as of `ricochet-core` 0.17.0.

Only `[content]` and `[language]` are required.
Every other table is optional, and several are rejected outright for the wrong content type.

## `[content]`

| Key            | Type             | Required | Notes                                                                                                 |
| -------------- | ---------------- | -------- | ----------------------------------------------------------------------------------------------------- |
| `id`           | string           | no       | The item ULID. Leave it out for a new item. The first successful deploy writes it back into the file. |
| `name`         | string           | yes      | Display name, 120 characters or fewer.                                                                |
| `slug`         | string           | no       | URL slug. The server derives one from the name when this is absent.                                   |
| `entrypoint`   | path             | yes      | Relative to the deployed directory. Allowed extensions depend on `content_type`.                      |
| `access_type`  | enum             | yes      | `private`, `internal`, or `external`.                                                                 |
| `content_type` | enum             | yes      | See the content type table below.                                                                     |
| `summary`      | string           | no       | Short description shown in the UI.                                                                    |
| `thumbnail`    | path             | no       | Relative path to a thumbnail image.                                                                   |
| `tags`         | array of strings | no       | Free-form tags.                                                                                       |
| `include`      | array of globs   | no       | When set, only matching paths are bundled.                                                            |
| `exclude`      | array of globs   | no       | Matching paths are dropped after `include` is applied.                                                |
| `exec_env`     | string           | no       | Named execution environment, for servers with a Docker or Kubernetes backend.                         |

`.venv`, `.renv`, and `__pycache__` are always excluded, at any nesting depth, even when an `include` pattern names them.

## `[language]`

| Key        | Type | Required | Notes                                                      |
| ---------- | ---- | -------- | ---------------------------------------------------------- |
| `name`     | enum | yes      | `r`, `python`, or `julia`. Must agree with `content_type`. |
| `packages` | enum | yes      | `renv.lock`, `uv.lock`, or `Manifest.toml`.                |

Python items must use `packages = "uv.lock"`; any other value fails validation with `Package file required`.
The named lockfile must exist in the deployed directory, or in a parent directory for `uv.lock`.

## `[schedule]`

Valid only for task content types.
Declaring it on an app fails validation with `Chosen content type cannot be scheduled`.

| Key             | Type             | Default | Notes                                                                                                                                |
| --------------- | ---------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `cron`          | string           | none    | A cron expression such as `0 9 * * 1-5`, or a shorthand such as `@hourly`, `@daily`, or `@weekly`. Validated locally at deploy time. |
| `parameterized` | bool             | `false` | Whether the task accepts parameters at invocation.                                                                                   |
| `on_success`    | array of strings | none    | Notification targets for a successful run.                                                                                           |
| `on_error`      | array of strings | none    | Notification targets for a failed run.                                                                                               |

## `[serve]`

Valid only for app content types.
Declaring it on a task fails with `Chosen content type cannot be served.`
The four sizing keys have no serde defaults, so all of them must be present once the table exists.

| Key                  | Type            | Suggested value | Notes                                                                                       |
| -------------------- | --------------- | --------------- | ------------------------------------------------------------------------------------------- |
| `min_instances`      | integer         | `0`             | Instances started eagerly.                                                                  |
| `max_instances`      | integer         | `5`             | Hard ceiling on concurrent instances. Clamped up to `min_instances` when lower.             |
| `spawn_threshold`    | integer         | `80`            | Percentage of `max_connections` at which a new instance spawns. Clamped to 100.             |
| `max_connections`    | integer         | `10`            | Connections a single instance will accept.                                                  |
| `max_connection_age` | integer seconds | unset           | Maximum lifetime of an instance. Unset means instances end when the last connection closes. |
| `inactive_timeout`   | integer seconds | unset           | Idle time before a connection is closed.                                                    |
| `connection_timeout` | integer seconds | unset           | Time allowed to establish a connection.                                                     |

### `[serve.k8s]`

| Key               | Type   | Default   | Notes                                                   |
| ----------------- | ------ | --------- | ------------------------------------------------------- |
| `strategy`        | enum   | `rolling` | `rolling` or `recreate`.                                |
| `max_surge`       | string | unset     | Count or percentage, such as `1` or `25%`.              |
| `max_unavailable` | string | unset     | Count or percentage, such as `0` or `10%`.              |
| `config`          | string | unset     | Raw backend configuration passed through to Kubernetes. |

## `[static]`

Valid only for content types that can produce static output, which is the same set as the task types.
Declaring it on an app fails with `Chosen content type cannot be served as a static site.`

| Key          | Type   | Notes                                                                      |
| ------------ | ------ | -------------------------------------------------------------------------- |
| `index`      | string | File served at the item root, conventionally `index.html`.                 |
| `output_dir` | string | Directory holding the rendered output, relative to the deployed directory. |
| `render_fn`  | string | Custom render function, when the default renderer is not used.             |

`ricochet init` fills both keys automatically for a Quarto project by reading `project.output-dir` from `_quarto.yml`.

## `[resources]`

Infrastructure-agnostic limits, applied by the Docker and Kubernetes backends.
Unset keys fall back to the server-wide defaults.

| Key               | Type            | Example   | Notes                                                             |
| ----------------- | --------------- | --------- | ----------------------------------------------------------------- |
| `cpu_request`     | string          | `"500m"`  | Millicores, or a plain number of cores.                           |
| `cpu_limit`       | string          | `"1000m"` | Same format as `cpu_request`.                                     |
| `memory_request`  | string          | `"256Mi"` | `Ki`, `Mi`, or `Gi` suffix, or plain bytes.                       |
| `memory_limit`    | string          | `"512Mi"` | Same format as `memory_request`.                                  |
| `restore_timeout` | integer seconds | `1800`    | Timeout for the dependency restore job. `0` disables the timeout. |

## `[repositories]`

Package sources, keyed by language.
R takes a named map, while Python and Julia take arrays of URLs.

| Key               | Type                 | Default | Notes                                                                                                         |
| ----------------- | -------------------- | ------- | ------------------------------------------------------------------------------------------------------------- |
| `r`               | table of name to URL | unset   | For example `{ CRAN = "https://cloud.r-project.org" }`. Unnamed lists are ignored with a warning.             |
| `python`          | array of URLs        | unset   | Named maps are ignored with a warning.                                                                        |
| `julia`           | array of URLs        | unset   | Named maps are ignored with a warning.                                                                        |
| `allow_overrides` | bool                 | `true`  | Read from the server configuration. When the server sets it to false, these item-level sources are discarded. |

Item sources are merged into the server sources, and the item wins on a name collision.

## `[retention]`

Per-item bundle retention.
An item can only tighten the server-wide policy, never relax it, because the resolver takes the lower of the two values.

| Key               | Type      | Default | Notes                                  |
| ----------------- | --------- | ------- | -------------------------------------- |
| `max_age_days`    | integer   | unset   | Delete deployments older than this.    |
| `max_deployments` | integer   | `20`    | Deployments kept per item.             |
| `max_bundle_size` | byte size | `100MB` | Largest bundle the server will accept. |

## `[env_vars]`

Written by the CLI, never by hand.
It holds the AES-encrypted payload produced by `ricochet deploy -e` as `nonce` and `vars`, both base64.
Manage variables with `ricochet app env-vars` or `ricochet task env-vars` instead of editing this table.

## Content types

`task` types are invokable, schedulable, and can be served as static output.
`app` types are served live and reject `[schedule]` and `[static]`.

| `content_type`   | `language.name` | Kind | Entrypoint                                                |
| ---------------- | --------------- | ---- | --------------------------------------------------------- |
| `r`              | `r`             | task | `.R`                                                      |
| `rmd`            | `r`             | task | `.Rmd` or `.R`                                            |
| `quarto-r`       | `r`             | task | `.qmd`, `.Rmd`, `.R`, or `_quarto.yml`                    |
| `julia`          | `julia`         | task | `.jl`                                                     |
| `quarto-jl`      | `julia`         | task | `.qmd` or `_quarto.yml`                                   |
| `python`         | `python`        | task | `.py`                                                     |
| `quarto-py`      | `python`        | task | `.qmd` or `_quarto.yml`                                   |
| `shiny`          | `r`             | app  | A directory holding `ui.R` and `server.R`, or a `.R` file |
| `rmd-shiny`      | `r`             | app  | `.Rmd` or `.R`                                            |
| `quarto-r-shiny` | `r`             | app  | `.qmd`, `.Rmd`, `.R`, or `_quarto.yml`                    |
| `plumber`        | `r`             | app  | Any `.R` file                                             |
| `ambiorix`       | `r`             | app  | `.R`                                                      |
| `r-service`      | `r`             | app  | `.R`                                                      |
| `serverless-r`   | `r`             | app  | `.R`                                                      |
| `julia-service`  | `julia`         | app  | `.jl`                                                     |
| `shiny-py`       | `python`        | app  | A directory, or a `.py` file                              |
| `python-service` | `python`        | app  | `.py`                                                     |
| `fast-api`       | `python`        | app  | `.py`                                                     |
| `flask`          | `python`        | app  | `.py`                                                     |
| `streamlit`      | `python`        | app  | `.py`                                                     |
| `dash`           | `python`        | app  | `.py`                                                     |

## Examples

A scheduled Quarto report rendered to a static site:

```toml
[content]
id = "01KE52BY41EQ7NE89K7Z5MMZ84"
name = "Weekly sales report"
entrypoint = "report.qmd"
access_type = "internal"
content_type = "quarto-r"
summary = "Sales rollup, refreshed every weekday morning."
tags = ["sales", "reporting"]
exclude = ["data/raw/**"]

[language]
name = "r"
packages = "renv.lock"

[schedule]
cron = "0 9 * * 1-5"

[static]
index = "index.html"
output_dir = "_site"

[retention]
max_deployments = 10
```

A Shiny app with sizing and resource limits:

```toml
[content]
name = "Fleet dashboard"
entrypoint = "app.R"
access_type = "external"
content_type = "shiny"

[language]
name = "r"
packages = "renv.lock"

[serve]
min_instances = 1
max_instances = 10
spawn_threshold = 75
max_connections = 20
inactive_timeout = 900

[resources]
cpu_limit = "1000m"
memory_limit = "2Gi"
```

A Python service on uv:

```toml
[content]
name = "scoring-api"
entrypoint = "main.py"
access_type = "internal"
content_type = "fast-api"

[language]
name = "python"
packages = "uv.lock"

[serve]
min_instances = 1
max_instances = 4
spawn_threshold = 80
max_connections = 50

[repositories]
python = ["https://pypi.org/simple"]
```
