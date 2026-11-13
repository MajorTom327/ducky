# ducky

A terminal explorer for PostgreSQL, inspired by Adminer. Browse schemas, tables, structure and data; run SQL scripts in a vim-style editor with highlighting and completion; store SQL files; dump and restore databases with `pg_dump` / `pg_restore`.

## Run

```sh
cargo run -p ducky-tui -- [--config PATH] [--profile LABEL] [--no-effects]
just dev            # uses dev/ducky.dev.toml against the compose database
```

`--profile` connects to the profile with that label on startup (unknown label: exit code 2). `--no-effects` disables the tachyonfx animations. A missing or invalid config prints the error, the searched paths and an example config, then exits with code 1.

## Configuration

Lookup order:

1. `--config PATH`
2. `./ducky.toml`
3. `$XDG_CONFIG_HOME/ducky/config.toml`, or `$HOME/.config/ducky/config.toml` when `XDG_CONFIG_HOME` is unset

```toml
sql_dir = "sql"      # default "sql", relative to the config file
dump_dir = "dumps"   # default "dumps", relative to the config file

[[profiles]]
label = "Local"
url = "postgres://postgres:postgres@localhost:5432/postgres"
type = "local"

[[profiles]]
label = "Production"
url = "postgres://app:${PROD_DB_PASSWORD}@db.example.com:5432/app?sslmode=require"
type = "prod"
```

- `type` is `local` (green), `staging` (orange) or `prod` (red); the header and focus colours follow it.
- `${VAR}` in `url` is expanded from the environment at connect time; an unset variable fails the connection.
- Labels must be non-empty and unique; urls start with `postgres://` or `postgresql://`. Unknown keys are rejected.
- `sslmode`: `disable`; `prefer`/`allow`/absent; `require` (encrypted, unverified); `verify-ca`/`verify-full` (verified against native roots).

See `ducky.example.toml`.

## Dumps

Dump and restore need `pg_dump` and `pg_restore` in `PATH` (macOS: `brew install libpq`, Debian/Ubuntu: `apt install postgresql-client`). When missing, the sidebar entries show `(unavailable)` and the views explain why. Dumps use the custom format; the password is passed through `PGPASSWORD`, never on the command line.

## Development

|Recipe|Does|
|---|---|
|`just build` / `just release`|build the workspace|
|`just fmt`|`cargo fmt --all`|
|`just lint`|fmt check + clippy with `-D warnings`|
|`just test`|`cargo test --workspace`|
|`just db-up` / `just db-down`|start / remove the Postgres 18 compose database on port 5440 (seeded from `dev/seed.sql`)|
|`just test-db`|`db-up`, then the ignored database tests with `DUCKY_TEST_DATABASE_URL`|
|`just dev *args`|run ducky against `dev/ducky.dev.toml`|
|`just ci`|lint, test, release|

CI (`.gitea/workflows/ci.yml`) runs on pushes to `main` and pull requests: fmt check, clippy, tests, ignored database tests against a `postgres:18-alpine` service (skipping `dump_then_restore`, the runner has no PostgreSQL 18 client tools), release build.

Docs: [architecture](docs/architecture.md), [key bindings](docs/keybindings.md). Each crate has its own README.
