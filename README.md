# ducky

A terminal explorer for PostgreSQL, inspired by Adminer. Browse schemas, tables, structure and data; run SQL scripts in a vim-style editor with highlighting and completion; store SQL files; dump and restore databases with `pg_dump` / `pg_restore`.

## Run

```sh
cargo run -p ducky-tui -- [--config PATH] [--profile LABEL|PROJECT/LABEL] [--no-effects]
just dev            # uses dev/ducky.dev.toml against the compose database
```

`--profile` connects on startup to the profile matching `LABEL`, `PROJECT/LABEL` or `PROJECT/TYPE` (unknown profile: the profiles are listed grouped by project; several matches: the matching profiles are listed; exit code 2 in both cases). `--no-effects` disables the tachyonfx animations. A missing or invalid config prints the error, the searched paths and an example config, then exits with code 1.

## Configuration

Lookup order:

1. `--config PATH`
2. `./ducky.toml`
3. `$XDG_CONFIG_HOME/ducky/config.toml`, or `$HOME/.config/ducky/config.toml` when `XDG_CONFIG_HOME` is unset

```toml
sql_dir = "sql"      # default "sql", relative to the config file
dump_dir = "dumps"   # default "dumps", relative to the config file
page_size = 50       # rows per table page, 1–5000, default 50

[[profiles]]
label = "Shop local"
project = "shop"         # optional, groups profiles in the picker
type = "local"
url = "postgres://postgres:postgres@localhost:5432/shop"

[[profiles]]
project = "shop"
type = "prod"
tags = ["eu", "readonly"]        # optional, extra words to filter on
url = "postgres://app:${PROD_DB_PASSWORD}@prod.example.com:5432/shop?sslmode=require"
```

- `project` (optional) groups profiles: the picker lists projects in order of first appearance, each with its profiles in file order, then profiles without a project under `Other`. A blank project counts as none.
- `tags` (optional) are trimmed and lowercased, duplicates dropped; an empty tag is rejected. The picker shows them as `#eu #readonly`.
- `type` is `local` (green), `staging` (orange) or `prod` (red); the header and focus colours follow it.
- `${VAR}` in `url` is expanded from the environment at connect time; an unset variable fails the connection.
- `label` is optional when `project` is set: the profile is then named `<project> <type>` (`shop prod`) in notices and confirmations, and its picker row shows only the badge and target. A profile needs a label or a project; labels may repeat; urls start with `postgres://` or `postgresql://`. Unknown keys are rejected. `page_size` outside 1–5000 is rejected.
- `sslmode`: `disable`; `prefer`/`allow`/absent; `require` (encrypted, unverified); `verify-ca`/`verify-full` (verified against native roots).

See `ducky.example.toml`.

## Dumps

Dump and restore need `pg_dump` and `pg_restore` in `PATH` (macOS: `brew install libpq`, Debian/Ubuntu: `apt install postgresql-client`). When missing, the Dump / Restore tabs explain why and how to install them. Dumps use the custom format; the password is passed through `PGPASSWORD`, never on the command line.

## Usage

Startup opens the profile picker (` Connect `): a filter line, then the profiles grouped by project, each row with its type badge in the type colour, its label, its `#tags` and its target (`host:port/database` from the url, `${VAR}` left as written). Typing filters: every space-separated word must appear in the project, label, type, tags or target (`shop prod`, `eu`, `5440`). `↓`/`↑`, `Tab`, `Ctrl+j`/`Ctrl+k` move, `Enter` connects; `Esc` leaves the filter for the list, where `j`/`k`, `g`/`G` move and `/` or `i` return to the filter.

Every main view opens in a tab: one tab per opened relation (reused when reopened), numbered `Query N` SQL tabs (`Ctrl+n`), and single `SQL files`, `Dump` and `Restore` tabs. The sidebar lists the relations of the current schema with their estimated row count or kind. `Ctrl+p` opens "go to anything": open tabs, actions (including `Switch profile…`) and the relations of every schema. Only one script runs at a time across all query tabs. See [key bindings](docs/keybindings.md).

Quit with `Ctrl+q` anywhere, `q` outside the editor, the `Space q q` sequence, or `:q` (`:` opens go to anything with `:` typed, outside editor insert mode).

To run the same query on another database, open the same picker as a popup with `Space p` or `Switch profile…` in go to anything (the connected profile is marked `●`), filter and press `Enter`: the workspace reconnects to that profile and keeps every tab, query text and result; the catalog and table tabs reload, and the results title shows the profile each script ran on (` · on Dev local `). Switching waits for running scripts, dumps and restores to finish; a failed connection keeps the current profile. The header shows the project before the profile (` ducky │ shop │ [PROD] Shop prod │ … `).

Foreign keys are navigable from table data (foreign-key columns end with ` →` in the header). On a foreign-key cell, `gd` (go to referenced row) opens the referenced table in a new tab filtered on that key (`where "id" = '42'`). `gr` (find referencing rows) opens, in a new tab, the rows of another table whose foreign key points at the current row; when several foreign keys reference the table, a picker lists them (`orders.customer_id → customers.id`). A tab with the same table and filter is reused, and the sidebar follows the schema of the opened table.

Rows of the current table page can be selected for an edit: `v` marks or unmarks the cursor row and moves down, `V` starts a visual range (a second `V` keeps it), `Esc` clears the selection. Selected rows show `▌` in the gutter and the bottom border counts them. `e` opens a new query tab with an `UPDATE` preset for the selected rows (the cursor row when none is selected), targeting them by primary key (or an `id` column) and setting the cursor column to a `:<column>` parameter; `F5` asks for its value through the query parameters form:

```sql
UPDATE "public"."orders"
SET
  "status" = :status
WHERE "id" IN ('17', '21', '25')
;
```

Tables with an `id` column open sorted on `id` descending, newest first with UUIDv7 or serial keys (`s` cycles the sort). Single cells can also be changed in place, Drizzle Studio style: `c` on a cell opens an inline editor over it (a dropdown for enum columns, an immediate `t` / `f` / NULL toggle for booleans, a text input otherwise) and applies to every selected row. Changes stay as drafts, shown in bold yellow and counted as `N unsaved` in the bottom border, until `Ctrl+s` saves them all in one transaction (with a confirmation on `staging` and `prod`); `x` sets the cell (or the selected rows) to NULL, `u` reverts the cursor cell, `U` discards every change. Pages cannot be changed, filtered, reloaded or closed while drafts are pending. Views and `geometry` / `geography` columns are not editable. Saving sends one statement per edited row:

```sql
UPDATE "public"."orders" SET "status" = 'paid', "total" = NULL WHERE "id" = '5';
```

When browsing a table, PostGIS `geometry` and `geography` columns are shown as GeoJSON (`{"type":"Point","coordinates":[2.35,48.85]}`) in the grid, the row detail and copied values. Query tabs show exactly what your SQL returns; use `ST_AsGeoJSON` or `ST_AsText` there. Long values use the free width of the grid before being cut with `…`.

## Query parameters

A query may contain named parameters (`:name`) and positional ones (`$1`, `$2`, …):

```sql
select * from orders where status = :status and total > :min_total limit $1
```

Running it (`F5`, or executing a SQL file) opens a form with one field per parameter, in order of first appearance; the same name used twice is one field. A value is inserted as a string literal (`pending` → `'pending'`, quotes doubled; PostgreSQL coerces it, so `'100'` works for numbers and `limit '10'` too). `Ctrl+r` switches the selected field to raw SQL, inserted verbatim (`now()`, `NULL`, `(1, 2, 3)`); each row previews what will be inserted. `Enter` runs, `Esc` cancels. `::` casts, `arr[1:n]` slices, strings, quoted identifiers, comments and dollar-quoted bodies are not parameters.

Each query tab remembers the last values, so after switching profile the same query reruns with the form prefilled. Results show the SQL that actually ran, with the values in place.

## Run confirmation

What happens on `F5` / `Ctrl+e` (and on `x` in SQL files) depends on the type of the connected profile:

|Type|Before running|
|---|---|
|`local`|never asks|
|`staging`|asks before the first run of each query tab; once confirmed, later runs of that tab run directly. Switching to another profile and back asks again|
|`prod`|asks before every run|

The confirmation shows `Run <n> statement(s) on STAGING|PROD · <name> (<host:port/database>)?`, the first three lines of the SQL (with parameters already filled in) and whether it runs as one transaction or in autocommit, framed in the profile's colour. `y` / `Enter` runs, `n` / `Esc` cancels (`Run cancelled`). Restoring a dump always asks, whatever the profile.

## Development

|Recipe|Does|
|---|---|
|`just build` / `just release`|build the workspace|
|`just fmt`|`cargo fmt --all`|
|`just lint`|fmt check + clippy with `-D warnings`|
|`just test`|`cargo test --workspace`|
|`just db-up` / `just db-down`|start / remove the PostgreSQL 18 + PostGIS compose database on port 5440 (seeded from `dev/seed.sql`)|
|`just test-db`|`db-up`, then the ignored database tests with `DUCKY_TEST_DATABASE_URL`|
|`just coverage`|`db-up`, every test (including ignored) under `cargo llvm-cov`, HTML report in `target/coverage/html`, fails below 97% line coverage|
|`just dev *args`|run ducky against `dev/ducky.dev.toml`|
|`just ci`|lint, test, release|

CI (`.gitea/workflows/ci.yml`) runs on pushes to `main` and pull requests: fmt check, clippy, then every test including the ignored database tests under `cargo llvm-cov`, against an `imresamu/postgis:18-3.6-alpine` service (skipping `dump_then_restore`, the runner has no PostgreSQL 18 client tools), and a release build. The run uploads a `coverage` artifact (HTML report + `lcov.info`), prints the per-file summary in the log, and fails below 97% line coverage.

Docs: [architecture](docs/architecture.md), [key bindings](docs/keybindings.md). Each crate has its own README.
