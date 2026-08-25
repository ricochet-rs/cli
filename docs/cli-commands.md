# Command-Line Help for `ricochet`

This document contains the help content for the `ricochet` command-line program.

**Command Overview:**

* [`ricochet`↴](#ricochet)
* [`ricochet login`↴](#ricochet-login)
* [`ricochet logout`↴](#ricochet-logout)
* [`ricochet deploy`↴](#ricochet-deploy)
* [`ricochet delete`↴](#ricochet-delete)
* [`ricochet config`↴](#ricochet-config)
* [`ricochet init`↴](#ricochet-init)
* [`ricochet app`↴](#ricochet-app)
* [`ricochet app toml`↴](#ricochet-app-toml)
* [`ricochet app list`↴](#ricochet-app-list)
* [`ricochet app instances`↴](#ricochet-app-instances)
* [`ricochet app stop`↴](#ricochet-app-stop)
* [`ricochet app settings`↴](#ricochet-app-settings)
* [`ricochet app settings update`↴](#ricochet-app-settings-update)
* [`ricochet app deployment`↴](#ricochet-app-deployment)
* [`ricochet app deployment list`↴](#ricochet-app-deployment-list)
* [`ricochet app deployment get`↴](#ricochet-app-deployment-get)
* [`ricochet app env-vars`↴](#ricochet-app-env-vars)
* [`ricochet app env-vars get`↴](#ricochet-app-env-vars-get)
* [`ricochet app env-vars delete`↴](#ricochet-app-env-vars-delete)
* [`ricochet app env-vars set`↴](#ricochet-app-env-vars-set)
* [`ricochet app env-vars replace`↴](#ricochet-app-env-vars-replace)
* [`ricochet task`↴](#ricochet-task)
* [`ricochet task list`↴](#ricochet-task-list)
* [`ricochet task toml`↴](#ricochet-task-toml)
* [`ricochet task invoke`↴](#ricochet-task-invoke)
* [`ricochet task schedule`↴](#ricochet-task-schedule)
* [`ricochet task settings`↴](#ricochet-task-settings)
* [`ricochet task settings update`↴](#ricochet-task-settings-update)
* [`ricochet task deployment`↴](#ricochet-task-deployment)
* [`ricochet task deployment list`↴](#ricochet-task-deployment-list)
* [`ricochet task deployment get`↴](#ricochet-task-deployment-get)
* [`ricochet task env-vars`↴](#ricochet-task-env-vars)
* [`ricochet task env-vars get`↴](#ricochet-task-env-vars-get)
* [`ricochet task env-vars delete`↴](#ricochet-task-env-vars-delete)
* [`ricochet task env-vars set`↴](#ricochet-task-env-vars-set)
* [`ricochet task env-vars replace`↴](#ricochet-task-env-vars-replace)
* [`ricochet server`↴](#ricochet-server)
* [`ricochet server list`↴](#ricochet-server-list)
* [`ricochet server add`↴](#ricochet-server-add)
* [`ricochet server remove`↴](#ricochet-server-remove)
* [`ricochet server set-default`↴](#ricochet-server-set-default)
* [`ricochet user`↴](#ricochet-user)
* [`ricochet user credentials`↴](#ricochet-user-credentials)
* [`ricochet self`↴](#ricochet-self)
* [`ricochet self update`↴](#ricochet-self-update)

## `ricochet`

Ricochet CLI

**Usage:** `ricochet [OPTIONS] [COMMAND]`

###### **Subcommands:**

* `login` — Authenticate with a Ricochet server
* `logout` — Remove stored credentials
* `deploy` — Deploy content to a Ricochet server
* `delete` — Delete a content item
* `config` — Show configuration
* `init` — Initialize a new Ricochet deployment
* `app` — Manage deployed app items
* `task` — Manage deployed task items
* `server` — Manage configured Ricochet servers
* `user` — Manage the current user's account
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
* `-e`, `--env <KEY[=VALUE]>` — Set an environment variable on the initial deployment. `KEY=VALUE` sets it directly; `KEY` alone resolves the value from .env, .Renviron, or the calling environment. Repeatable
* `--git <GIT>` — Deploy from a Git repository instead of a local bundle
* `--branch <BRANCH>` — Git branch to deploy (only with --git)
* `--path <REPO_PATH>` — Subdirectory within the Git repo containing _ricochet.toml (only with --git)
* `--config <CONFIG_PATH>` — Path to a local _ricochet.toml to use instead of the one in the repo (only with --git)
* `--credential <CREDENTIAL>` — Git credential ID to use for private repos (only with --git)



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
* `list` — List deployed app content items
* `instances` — List running instances
* `stop` — Stop a running instance, or all instances if no instance ID is given
* `settings` — Show the diff between the local _ricochet.toml and the deployed item. Use the `update` subcommand to apply it
* `deployment` — Manage deployments for an app
* `env-vars` — Manage environment variables for an app



## `ricochet app toml`

Fetch the remote _ricochet.toml for an item

**Usage:** `ricochet app toml [OPTIONS] [ID]`

###### **Arguments:**

* `<ID>` — Content item ID (ULID). If not provided, will read from local _ricochet.toml

###### **Options:**

* `-p`, `--path <PATH>` — Path to _ricochet.toml file



## `ricochet app list`

List deployed app content items

**Usage:** `ricochet app list [OPTIONS]`

###### **Options:**

* `-t`, `--content-type <CONTENT_TYPE>` — Filter by content type
* `-a`, `--active-only` — Show only active deployments (status: deployed, running, or success)
* `--all` — List every item on the instance, not just your own (requires an instance admin API key)
* `-s`, `--sort <SORT>` — Sort by field(s) - comma-separated for multiple (e.g., "name,updated" or "status,name") Prefix with '-' for descending order (e.g., "-updated,name")



## `ricochet app instances`

List running instances

**Usage:** `ricochet app instances [OPTIONS] [ID]`

###### **Arguments:**

* `<ID>` — Content item ID (ULID). If not provided, will read from local _ricochet.toml

###### **Options:**

* `-p`, `--path <PATH>` — Path to _ricochet.toml file



## `ricochet app stop`

Stop a running instance, or all instances if no instance ID is given

**Usage:** `ricochet app stop [OPTIONS] [ID] [PID]`

###### **Arguments:**

* `<ID>` — Content item ID (ULID). If not provided, will read from local _ricochet.toml
* `<PID>` — Instance ID to stop. If not provided, stops all instances

###### **Options:**

* `-p`, `--path <PATH>` — Path to _ricochet.toml file



## `ricochet app settings`

Show the diff between the local _ricochet.toml and the deployed item. Use the `update` subcommand to apply it

**Usage:** `ricochet app settings [OPTIONS] [COMMAND]`

###### **Subcommands:**

* `update` — Apply local _ricochet.toml settings to the server

###### **Options:**

* `-p`, `--path <PATH>` — Path to _ricochet.toml file



## `ricochet app settings update`

Apply local _ricochet.toml settings to the server

**Usage:** `ricochet app settings update [OPTIONS]`

###### **Options:**

* `-p`, `--path <PATH>` — Path to _ricochet.toml file
* `-f`, `--force` — Skip the confirmation prompt



## `ricochet app deployment`

Manage deployments for an app

**Usage:** `ricochet app deployment <COMMAND>`

###### **Subcommands:**

* `list` — List deployments for a content item
* `get` — Get a specific deployment



## `ricochet app deployment list`

List deployments for a content item

**Usage:** `ricochet app deployment list [OPTIONS] <ID>`

###### **Arguments:**

* `<ID>` — Content item ID (ULID)

###### **Options:**

* `--fields <FIELDS>` — Fields to display: 'all' or comma-separated names (id, status, deployed_at, deployed_by, content_id, requested_ver, matched_ver, git_hash)



## `ricochet app deployment get`

Get a specific deployment

**Usage:** `ricochet app deployment get <ID>`

###### **Arguments:**

* `<ID>` — Deployment ID (ULID)



## `ricochet app env-vars`

Manage environment variables for an app

**Usage:** `ricochet app env-vars <COMMAND>`

###### **Subcommands:**

* `get` — List the names of an item's environment variables
* `delete` — Delete an environment variable by name
* `set` — Set (upsert) environment variables, leaving others untouched
* `replace` — Replace all environment variables (any not listed are deleted)



## `ricochet app env-vars get`

List the names of an item's environment variables

**Usage:** `ricochet app env-vars get [OPTIONS] [ID]`

###### **Arguments:**

* `<ID>` — Content item ID (ULID). If not provided, will read from local _ricochet.toml

###### **Options:**

* `-p`, `--path <PATH>` — Path to _ricochet.toml file



## `ricochet app env-vars delete`

Delete an environment variable by name

**Usage:** `ricochet app env-vars delete [OPTIONS] <NAME>`

###### **Arguments:**

* `<NAME>` — Environment variable name

###### **Options:**

* `-i`, `--id <ID>` — Content item ID (ULID). If not provided, will read from local _ricochet.toml
* `-p`, `--path <PATH>` — Path to _ricochet.toml file
* `-f`, `--force` — Skip confirmation



## `ricochet app env-vars set`

Set (upsert) environment variables, leaving others untouched

**Usage:** `ricochet app env-vars set [OPTIONS] <KEY[=VALUE]>...`

###### **Arguments:**

* `<KEY[=VALUE]>` — Variables to set. `KEY=VALUE` sets it directly. `KEY` alone resolves the value from .env, .Renviron, or the calling environment

###### **Options:**

* `-i`, `--id <ID>` — Content item ID (ULID). If not provided, will read from local _ricochet.toml
* `-p`, `--path <PATH>` — Path to _ricochet.toml file



## `ricochet app env-vars replace`

Replace all environment variables (any not listed are deleted)

**Usage:** `ricochet app env-vars replace [OPTIONS] [KEY[=VALUE]]...`

###### **Arguments:**

* `<KEY[=VALUE]>` — Variables to set. `KEY=VALUE` sets it directly. `KEY` alone resolves the value from .env, .Renviron, or the calling environment. Omit entirely to delete all environment variables

###### **Options:**

* `-i`, `--id <ID>` — Content item ID (ULID). If not provided, will read from local _ricochet.toml
* `-p`, `--path <PATH>` — Path to _ricochet.toml file
* `-f`, `--force` — Skip confirmation



## `ricochet task`

Manage deployed task items

**Usage:** `ricochet task <COMMAND>`

###### **Subcommands:**

* `list` — List deployed task content items
* `toml` — Fetch the remote _ricochet.toml for a task
* `invoke` — Invoke a task
* `schedule` — Set or update the schedule for a task
* `settings` — Show the diff between the local _ricochet.toml and the deployed item. Use the `update` subcommand to apply it
* `deployment` — Manage deployments for a task
* `env-vars` — Manage environment variables for a task



## `ricochet task list`

List deployed task content items

**Usage:** `ricochet task list [OPTIONS]`

###### **Options:**

* `-t`, `--content-type <CONTENT_TYPE>` — Filter by content type
* `-a`, `--active-only` — Show only active deployments (status: deployed, running, or success)
* `--all` — List every item on the instance, not just your own (requires an instance admin API key)
* `-s`, `--sort <SORT>` — Sort by field(s) - comma-separated for multiple (e.g., "name,updated" or "status,name") Prefix with '-' for descending order (e.g., "-updated,name")



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



## `ricochet task settings`

Show the diff between the local _ricochet.toml and the deployed item. Use the `update` subcommand to apply it

**Usage:** `ricochet task settings [OPTIONS] [COMMAND]`

###### **Subcommands:**

* `update` — Apply local _ricochet.toml settings to the server

###### **Options:**

* `-p`, `--path <PATH>` — Path to _ricochet.toml file



## `ricochet task settings update`

Apply local _ricochet.toml settings to the server

**Usage:** `ricochet task settings update [OPTIONS]`

###### **Options:**

* `-p`, `--path <PATH>` — Path to _ricochet.toml file
* `-f`, `--force` — Skip the confirmation prompt



## `ricochet task deployment`

Manage deployments for a task

**Usage:** `ricochet task deployment <COMMAND>`

###### **Subcommands:**

* `list` — List deployments for a content item
* `get` — Get a specific deployment



## `ricochet task deployment list`

List deployments for a content item

**Usage:** `ricochet task deployment list [OPTIONS] <ID>`

###### **Arguments:**

* `<ID>` — Content item ID (ULID)

###### **Options:**

* `--fields <FIELDS>` — Fields to display: 'all' or comma-separated names (id, status, deployed_at, deployed_by, content_id, requested_ver, matched_ver, git_hash)



## `ricochet task deployment get`

Get a specific deployment

**Usage:** `ricochet task deployment get <ID>`

###### **Arguments:**

* `<ID>` — Deployment ID (ULID)



## `ricochet task env-vars`

Manage environment variables for a task

**Usage:** `ricochet task env-vars <COMMAND>`

###### **Subcommands:**

* `get` — List the names of an item's environment variables
* `delete` — Delete an environment variable by name
* `set` — Set (upsert) environment variables, leaving others untouched
* `replace` — Replace all environment variables (any not listed are deleted)



## `ricochet task env-vars get`

List the names of an item's environment variables

**Usage:** `ricochet task env-vars get [OPTIONS] [ID]`

###### **Arguments:**

* `<ID>` — Content item ID (ULID). If not provided, will read from local _ricochet.toml

###### **Options:**

* `-p`, `--path <PATH>` — Path to _ricochet.toml file



## `ricochet task env-vars delete`

Delete an environment variable by name

**Usage:** `ricochet task env-vars delete [OPTIONS] <NAME>`

###### **Arguments:**

* `<NAME>` — Environment variable name

###### **Options:**

* `-i`, `--id <ID>` — Content item ID (ULID). If not provided, will read from local _ricochet.toml
* `-p`, `--path <PATH>` — Path to _ricochet.toml file
* `-f`, `--force` — Skip confirmation



## `ricochet task env-vars set`

Set (upsert) environment variables, leaving others untouched

**Usage:** `ricochet task env-vars set [OPTIONS] <KEY[=VALUE]>...`

###### **Arguments:**

* `<KEY[=VALUE]>` — Variables to set. `KEY=VALUE` sets it directly. `KEY` alone resolves the value from .env, .Renviron, or the calling environment

###### **Options:**

* `-i`, `--id <ID>` — Content item ID (ULID). If not provided, will read from local _ricochet.toml
* `-p`, `--path <PATH>` — Path to _ricochet.toml file



## `ricochet task env-vars replace`

Replace all environment variables (any not listed are deleted)

**Usage:** `ricochet task env-vars replace [OPTIONS] [KEY[=VALUE]]...`

###### **Arguments:**

* `<KEY[=VALUE]>` — Variables to set. `KEY=VALUE` sets it directly. `KEY` alone resolves the value from .env, .Renviron, or the calling environment. Omit entirely to delete all environment variables

###### **Options:**

* `-i`, `--id <ID>` — Content item ID (ULID). If not provided, will read from local _ricochet.toml
* `-p`, `--path <PATH>` — Path to _ricochet.toml file
* `-f`, `--force` — Skip confirmation



## `ricochet server`

Manage configured Ricochet servers

**Usage:** `ricochet server <COMMAND>`

###### **Subcommands:**

* `list` — List all configured servers
* `add` — Add a new server
* `remove` — Remove a server
* `set-default` — Set the default server



## `ricochet server list`

List all configured servers

**Usage:** `ricochet server list`



## `ricochet server add`

Add a new server

**Usage:** `ricochet server add [OPTIONS] <NAME> <URL>`

###### **Arguments:**

* `<NAME>` — Server name (e.g., 'production', 'staging', 'local')
* `<URL>` — Server URL (must include http:// or https://)

###### **Options:**

* `--default` — Set this server as the default



## `ricochet server remove`

Remove a server

**Usage:** `ricochet server remove [OPTIONS] <NAME>`

###### **Arguments:**

* `<NAME>` — Server name to remove

###### **Options:**

* `-f`, `--force` — Skip confirmation prompt



## `ricochet server set-default`

Set the default server

**Usage:** `ricochet server set-default <NAME>`

###### **Arguments:**

* `<NAME>` — Server name to set as default



## `ricochet user`

Manage the current user's account

**Usage:** `ricochet user <COMMAND>`

###### **Subcommands:**

* `credentials` — List Git credentials



## `ricochet user credentials`

List Git credentials

**Usage:** `ricochet user credentials [OPTIONS]`

###### **Options:**

* `--user-id <USER_ID>` — Filter by user ID (admins only)
* `--type <TYPE>` — Filter by credential type

  Possible values: `ssh`, `https`




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
* `--dry-run` — Check for a newer version and report it without updating



<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>

