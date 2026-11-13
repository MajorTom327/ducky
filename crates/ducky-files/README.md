# ducky-files

SQL file storage, dump storage and `pg_dump` / `pg_restore` process handling for ducky.

## Public API

- `normalize_file_name(raw_name, extension)` — trims, rejects empty names, path separators, NUL and leading dots, appends the extension.
- `SqlStore::{new, list, load, preview, save}` — `*.sql` files in the SQL directory; `load` refuses files above `MAX_EDITOR_FILE_BYTES`, `preview` keeps the first `PREVIEW_LINE_LIMIT` lines.
- `DumpStore::{new, list, path_for}` and `default_dump_name(database, now)` — `*.dump` files, newest first.
- `detect_tools(path_variable)` — locates executable `pg_dump` / `pg_restore` in a `PATH` value.
- `CommandSpec`, `dump_command`, `restore_command` — build tool invocations; the password moves from the URL to `PGPASSWORD`.
- `run_tool(spec, log_lines)` — runs a tool, streams stderr lines, reports `ToolFailed` with the last line.
- `FilesError` — error enum.

## Tests

```sh
cargo test -p ducky-files
```
