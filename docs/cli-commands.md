# Command-Line Help for `ricochet`

This document contains the help content for the `ricochet` command-line program.

**Command Overview:**

* [`ricochet`↴](#ricochet)
* [`ricochet login`↴](#ricochet-login)
* [`ricochet logout`↴](#ricochet-logout)
* [`ricochet deploy`↴](#ricochet-deploy)
* [`ricochet list`↴](#ricochet-list)
* [`ricochet status`↴](#ricochet-status)
* [`ricochet invoke`↴](#ricochet-invoke)
* [`ricochet stop`↴](#ricochet-stop)
* [`ricochet delete`↴](#ricochet-delete)
* [`ricochet schedule`↴](#ricochet-schedule)
* [`ricochet settings`↴](#ricochet-settings)
* [`ricochet config`↴](#ricochet-config)

## `ricochet`

Ricochet CLI

**Usage:** `ricochet [OPTIONS] <COMMAND>`

###### **Subcommands:**

* `login` — Authenticate with the Ricochet server
* `logout` — Remove stored credentials
* `deploy` — Deploy content to the server
* `list` — List all content items
* `status` — Get status of a content item
* `invoke` — Invoke a content item
* `stop` — Stop a running service or invocation
* `delete` — Delete a content item
* `schedule` — Manage content schedules
* `settings` — Update content settings
* `config` — Show configuration

###### **Options:**

* `--server <SERVER>` — Server URL (can also be set with RICOCHET_SERVER environment variable)
* `--format <FORMAT>` — Output format

  Default value: `table`

  Possible values: `table`, `json`, `yaml`




## `ricochet login`

Authenticate with the Ricochet server

**Usage:** `ricochet login [OPTIONS]`

###### **Options:**

* `--api-key <API_KEY>` — API key (can also be provided interactively)



## `ricochet logout`

Remove stored credentials

**Usage:** `ricochet logout`



## `ricochet deploy`

Deploy content to the server

**Usage:** `ricochet deploy [OPTIONS] <PATH>`

###### **Arguments:**

* `<PATH>` — Path to the content directory or bundle

###### **Options:**

* `--name <NAME>` — Name for the deployment
* `--description <DESCRIPTION>` — Description for the deployment



## `ricochet list`

List all content items

**Usage:** `ricochet list [OPTIONS]`

###### **Options:**

* `--content-type <CONTENT_TYPE>` — Filter by content type
* `--active-only` — Show only active deployments



## `ricochet status`

Get status of a content item

**Usage:** `ricochet status <ID>`

###### **Arguments:**

* `<ID>` — Content item ID (ULID)



## `ricochet invoke`

Invoke a content item

**Usage:** `ricochet invoke [OPTIONS] <ID>`

###### **Arguments:**

* `<ID>` — Content item ID (ULID)

###### **Options:**

* `--params <PARAMS>` — Parameters as JSON



## `ricochet stop`

Stop a running service or invocation

**Usage:** `ricochet stop [OPTIONS] <ID>`

###### **Arguments:**

* `<ID>` — Content item ID (ULID)

###### **Options:**

* `--instance <INSTANCE>` — Instance PID (for services) or invocation ID



## `ricochet delete`

Delete a content item

**Usage:** `ricochet delete [OPTIONS] <ID>`

###### **Arguments:**

* `<ID>` — Content item ID (ULID)

###### **Options:**

* `--force` — Skip confirmation



## `ricochet schedule`

Manage content schedules

**Usage:** `ricochet schedule [OPTIONS] <ID>`

###### **Arguments:**

* `<ID>` — Content item ID (ULID)

###### **Options:**

* `--cron <CRON>` — Cron expression (e.g., "0 0 * * *" for daily at midnight)
* `--disable` — Disable the schedule



## `ricochet settings`

Update content settings

**Usage:** `ricochet settings --update <UPDATE> <ID>`

###### **Arguments:**

* `<ID>` — Content item ID (ULID)

###### **Options:**

* `--update <UPDATE>` — Update settings as JSON



## `ricochet config`

Show configuration

**Usage:** `ricochet config [OPTIONS]`

###### **Options:**

* `--show-all` — Show full configuration including sensitive values



<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>

