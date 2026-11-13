# ducky-runtime

Executes the `ducky_app::Command`s produced by `ducky_app::update` on tokio and answers with `ducky_app::Msg`s sent through an unbounded channel. It is the only crate that wires the pure state machine to the database (`ducky-postgres`), the file system and the PostgreSQL client tools (`ducky-files`), configuration (`ducky-config`) and the system clipboard (`arboard`).

## Public API

- `Runtime::new(storage: StoragePaths, messages: UnboundedSender<Msg>) -> Runtime`
- `Runtime::dispatch(&mut self, command: Command)`: must be called inside a tokio runtime. It never blocks: database and job work runs on spawned tasks, file-system work on `spawn_blocking`. `CopyToClipboard` is the only command handled synchronously.

## Behaviour

| Command | Answer |
|---|---|
| `Connect` | expands `${VAR}` in the profile URL, opens a `PgSession`, reads its info, detects `pg_dump` / `pg_restore` in `PATH`, replaces the active connection; `Connected` |
| `Disconnect` | drops the active connection; no answer |
| `LoadCatalog`, `LoadPage`, `LoadDetails` | `CatalogLoaded`, `PageLoaded`, `DetailsLoaded` |
| `RunScript` | splits with `ducky_sql::split_statements`; `ScriptFinished` carrying the command's `tab_id` |
| `CancelScript` | `Notified(Error)` only when cancelling fails |
| `ListSqlFiles`, `LoadFilePreview`, `OpenFile` | `SqlFilesListed`, `FilePreviewLoaded`, `FileOpened` |
| `SaveFile` | `FileSaved` with the normalized name; an existing file without overwrite is `SaveFailure::AlreadyExists` |
| `ListDumps` | `DumpsListed` |
| `StartDump` | `JobLogged` per `pg_dump` stderr line, then `JobFinished`; refuses to overwrite and deletes a partial file on failure |
| `StartRestore` | `JobLogged` per `pg_restore` stderr line, then `JobFinished` |
| `CopyToClipboard` | `Notified(Error "Clipboard unavailable: …")` on failure |

Database commands without an active connection answer with `Err("Not connected")` (`RunScript`: a `ScriptFinished` for the same `tab_id` with a report failing at statement 0). The clipboard is created on first use and kept alive for the process, because X11 loses the selection when it drops.

## Tests

```sh
cargo test -p ducky-runtime
DUCKY_TEST_DATABASE_URL=postgres://ducky:ducky@localhost:5440/ducky cargo test -p ducky-runtime -- --ignored
```

The ignored tests need a running database (`just db-up`); `dump_then_restore_roundtrip` also needs `pg_dump` and `pg_restore` in `PATH`.
