# ducky-app

Pure, view-agnostic state machine of ducky. It owns the application state, the messages a front-end or the runtime can send, and the commands the runtime must execute. No IO, no async, no terminal code: a TUI or a Dioxus front-end renders `AppState` and turns user input into `Msg`.

```text
front-end input ──Msg──▶ update(&mut AppState, Msg) ──Vec<Command>──▶ runtime ──Msg──▶ update …
```

## Public API

- `update(state: &mut AppState, msg: Msg) -> Vec<Command>`: the only state transition. Messages addressed to a screen, view or modal that is not active are ignored; responses for a request that is no longer current (`PageLoaded`, `DetailsLoaded`, `FilePreviewLoaded`, `Connected` without a pending connect) are dropped.
- `Loadable<T>` (`Idle`, `Loading`, `Ready`, `Failed`) with `ready()` and `is_loading()`.
- State: `AppState` (`new`, `workspace`), `Screen`, `ProfilePicker`, `Workspace` (`new`, `sidebar_items`, `schema_names`), `Focus`, `MainView`, `Sidebar`, `SidebarAction`, `SidebarItem`, `GridCursor`, `TableTab`, `TableView`, `Document`, `QueryPane`, `RunState`, `PendingSave`, `QueryView` (`selected_result`), `FilesView`, `JobKind`, `JobState`, `DumpView`, `RestoreView`, `NoticeLevel`, `Notice`, `Modal`, `PromptKind`, `Prompt`, `ConfirmKind`, `Confirm`, `RowDetail`.
- Constants: `PAGE_SIZE` (100 rows per table page), `MAX_JOB_LOG_LINES` (1000 lines kept per dump / restore log).
- Messages: `Msg`, `Connection`, `SaveFailure`, `ProfilesMsg`, `SidebarMsg`, `GridMove`, `TableMsg`, `QueryMsg`, `FilesMsg`, `DumpMsg`, `RestoreMsg`, `ModalMsg`.
- Commands: `Command` (connect, catalog / page / details loading, scripts and cancellation, SQL files, dumps, restores, clipboard).

## Behaviour highlights

- Connecting replaces the profile picker with a `Workspace`; reconnecting the same profile keeps the workspace and reloads the catalog and the open page.
- Refreshing a ready catalog keeps it visible; after a reload the schema falls back to the current one, then `public`, then the first schema.
- Table pages are read-only: pagination, a three-step sort cycle (ascending, descending, none), a raw `WHERE` filter, row detail and cell copy.
- Scripts run in `Transaction` (default) or `AutoCommit` mode; a finished script selects its last result and reloads the catalog when a statement affected rows.
- Saving an unnamed document prompts for a name and asks before overwriting an existing file.
- Dump and restore are refused with the tool's `Missing` reason when `pg_dump` / `pg_restore` are absent; a restore always asks for confirmation.

## Tests

```sh
cargo test -p ducky-app
```

One integration test file per update module lives in `tests/`, sharing fixtures from `tests/support/mod.rs`.
