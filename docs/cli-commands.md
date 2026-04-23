# Command-Line Help for `ricochet`

This document contains the help content for the `ricochet` command-line program.

**Command Overview:**

* [`ricochet`↴](#ricochet)
* [`ricochet login`↴](#ricochet-login)
* [`ricochet logout`↴](#ricochet-logout)
* [`ricochet deploy`↴](#ricochet-deploy)
* [`ricochet list`↴](#ricochet-list)
* [`ricochet delete`↴](#ricochet-delete)
* [`ricochet config`↴](#ricochet-config)
* [`ricochet init`↴](#ricochet-init)
* [`ricochet app`↴](#ricochet-app)
* [`ricochet app toml`↴](#ricochet-app-toml)
* [`ricochet task`↴](#ricochet-task)
* [`ricochet task toml`↴](#ricochet-task-toml)
* [`ricochet task invoke`↴](#ricochet-task-invoke)
* [`ricochet task schedule`↴](#ricochet-task-schedule)
* [`ricochet servers`↴](#ricochet-servers)
* [`ricochet servers list`↴](#ricochet-servers-list)
* [`ricochet servers add`↴](#ricochet-servers-add)
* [`ricochet servers remove`↴](#ricochet-servers-remove)
* [`ricochet servers set-default`↴](#ricochet-servers-set-default)
* [`ricochet self`↴](#ricochet-self)
* [`ricochet self update`↴](#ricochet-self-update)

## `ricochet`

Ricochet CLI

**Usage:** `ricochet [OPTIONS] [COMMAND]`

###### **Subcommands:**

* `login` — Authenticate with a Ricochet server
* `logout` — Remove stored credentials
* `deploy` — Deploy content to a Ricochet server
* `list` — List all content items
* `delete` — Delete a content item
* `config` — Show configuration
* `init` — Initialize a new Ricochet deployment
* `app` — Manage deployed app items
* `task` — Manage deployed task items
* `servers` — Manage configured Ricochet servers
* `self` — Manage the ricochet CLI itself

###### **Options:**

* `-S`, `--server <SERVER>` — Server URL (can also be set with RICOCHET_SERVER environment variable)
* `-F`, `--format <FORMAT>` — Output format

  Default value: `table`

  Possible values: `table`, `json`, `yaml`

* `--debug` — Enable debug output
* `-V`, `--version` — Print version



## `ricochet login`

Authenticate with a Ricochet server

**Usage:** `ricochet login [OPTIONS]`

###### **Options:**

* `-k`, `--api-key <API_KEY>` — API key (can also be provided interactively)



## `ricochet logout`

Remove stored credentials

**Usage:** `ricochet logout`



## `ricochet deploy`

Deploy content to a Ricochet server

**Usage:** `ricochet deploy [OPTIONS] [PATH]`

###### **Arguments:**

* `<PATH>` — Path to the content directory or bundle

  Default value: `.`

###### **Options:**

* `-n`, `--name <NAME>` — Name for the deployment
* `-d`, `--description <DESCRIPTION>` — Description for the deployment



## `ricochet list`

List all content items

**Usage:** `ricochet list [OPTIONS]`

###### **Options:**

* `-t`, `--content-type <CONTENT_TYPE>` — Filter by content type
* `-a`, `--active-only` — Show only active deployments (status: deployed, running, or success)
* `-s`, `--sort <SORT>` — Sort by field(s) - comma-separated for multiple (e.g., "name,updated" or "status,name") Prefix with '-' for descending order (e.g., "-updated,name")



## `ricochet delete`

Delete a content item

**Usage:** `ricochet delete [OPTIONS] <ID>`

###### **Arguments:**

* `<ID>` — Content item ID (ULID)

###### **Options:**

* `-f`, `--force` — Skip confirmation



## `ricochet config`

Show configuration

**Usage:** `ricochet config [OPTIONS]`

###### **Options:**

* `-A`, `--show-all` — Show full configuration including sensitive values



## `ricochet init`

Initialize a new Ricochet deployment

**Usage:** `ricochet init [OPTIONS] [PATH]`

###### **Arguments:**

* `<PATH>` — Directory to initialize (defaults to current directory)

  Default value: `.`

###### **Options:**

* `--overwrite` — Overwrite existing _ricochet.toml file without confirmation
* `--dry-run` — Preview the _ricochet.toml without saving to file



## `ricochet app`

Manage deployed app items

**Usage:** `ricochet app <COMMAND>`

###### **Subcommands:**

* `toml` — Fetch the remote _ricochet.toml for an item



## `ricochet app toml`

Fetch the remote _ricochet.toml for an item

**Usage:** `ricochet app toml [OPTIONS] [ID]`

###### **Arguments:**

* `<ID>` — Content item ID (ULID). If not provided, will read from local _ricochet.toml

###### **Options:**

* `-p`, `--path <PATH>` — Path to _ricochet.toml file



## `ricochet task`

Manage deployed task items

**Usage:** `ricochet task <COMMAND>`

###### **Subcommands:**

* `toml` — Fetch the remote _ricochet.toml for a task
* `invoke` — Invoke a task
* `schedule` — Set or update the schedule for a task



## `ricochet task toml`

Fetch the remote _ricochet.toml for a task

**Usage:** `ricochet task toml [OPTIONS] [ID]`

###### **Arguments:**

* `<ID>` — Content item ID (ULID). If not provided, will read from local _ricochet.toml

###### **Options:**

* `-p`, `--path <PATH>` — Path to _ricochet.toml file



## `ricochet task invoke`

Invoke a task

**Usage:** `ricochet task invoke <ID>`

###### **Arguments:**

* `<ID>` — Content item ID (ULID)



## `ricochet task schedule`

Set or update the schedule for a task

**Usage:** `ricochet task schedule <ID> <SCHEDULE>`

###### **Arguments:**

* `<ID>` — Content item ID (ULID)
* `<SCHEDULE>` — Cron expression (e.g. "0 9 * * 1-5" for weekdays at 9am)



## `ricochet servers`

Manage configured Ricochet servers

**Usage:** `ricochet servers <COMMAND>`

###### **Subcommands:**

* `list` — List all configured servers
* `add` — Add a new server
* `remove` — Remove a server
* `set-default` — Set the default server



## `ricochet servers list`

List all configured servers

**Usage:** `ricochet servers list`



## `ricochet servers add`

Add a new server

**Usage:** `ricochet servers add [OPTIONS] <NAME> <URL>`

###### **Arguments:**

* `<NAME>` — Server name (e.g., 'production', 'staging', 'local')
* `<URL>` — Server URL (must include http:// or https://)

###### **Options:**

* `--default` — Set this server as the default



## `ricochet servers remove`

Remove a server

**Usage:** `ricochet servers remove [OPTIONS] <NAME>`

###### **Arguments:**

* `<NAME>` — Server name to remove

###### **Options:**

* `-f`, `--force` — Skip confirmation prompt



## `ricochet servers set-default`

Set the default server

**Usage:** `ricochet servers set-default <NAME>`

###### **Arguments:**

* `<NAME>` — Server name to set as default



## `ricochet self`

Manage the ricochet CLI itself

**Usage:** `ricochet self <COMMAND>`

###### **Subcommands:**

* `update` — Update the ricochet CLI to the latest version



## `ricochet self update`

Update the ricochet CLI to the latest version

**Usage:** `ricochet self update [OPTIONS]`

###### **Options:**

* `-f`, `--force` — Force reinstall even if already on the latest version



<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>

