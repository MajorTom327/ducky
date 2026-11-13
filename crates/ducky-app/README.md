# ducky-app

Pure, view-agnostic state machine of ducky. It owns the application state, the messages a front-end or the runtime can send, and the commands the runtime must execute. No IO, no async, no terminal code: a TUI or a Dioxus front-end renders `AppState` and turns user input into `Msg`.

```text
front-end input ──Msg──▶ update(&mut AppState, Msg) ──Vec<Command>──▶ runtime ──Msg──▶ update …
```

## Public API

- `update(state: &mut AppState, msg: Msg) -> Vec<Command>`: the only state transition. Messages addressed to a screen, tab kind or modal that is not active are ignored; responses for a request that is no longer current (`PageLoaded`, `DetailsLoaded`, `FilePreviewLoaded`, `Connected` without a pending connect) are dropped.
- `Loadable<T>` (`Idle`, `Loading`, `Ready`, `Failed`) with `ready()` and `is_loading()`.
- State: `AppState` (`new`, `workspace`), `Screen`, `ProfilePicker`, `Workspace` (`new`, `active_tab`, `tab_index`, `sidebar_items`, `palette_items`, `schema_names`), `Focus`, `Sidebar`, `SidebarItem` (relation, kind, estimated row count), `TabId`, `Tab` (`title`), `TabContent` (`Table`, `Query`, `Files`, `Dump`, `Restore`), `GridCursor`, `TableTab`, `TableView`, `Document`, `QueryPane`, `RunState`, `PendingSave` (tab id, name, content), `QueryView` (`new`, `selected_result`), `FilesView`, `JobKind`, `JobState`, `DumpView`, `RestoreView`, `NoticeLevel`, `Notice`, `Modal`, `PromptKind`, `Prompt`, `ConfirmKind`, `Confirm`, `RowDetail`, `CommandPalette`, `PaletteItem`, `PaletteAction`.
- Constants: `PAGE_SIZE` (100 rows per table page), `MAX_JOB_LOG_LINES` (1000 lines kept per dump / restore log).
- Messages: `Msg`, `Connection`, `SaveFailure`, `ProfilesMsg`, `TabsMsg`, `SidebarMsg`, `GridMove`, `TableMsg`, `QueryMsg`, `FilesMsg`, `DumpMsg`, `RestoreMsg`, `ModalMsg`.
- Commands: `Command` (connect, catalog / page / details loading, scripts tagged with the tab that runs them and cancellation, SQL files, dumps, restores, clipboard).

## Behaviour highlights

- Every main view is a tab of the workspace. Table tabs are reused per relation, query tabs are numbered `Query N`, the SQL files, dump and restore tabs are singletons. Ids come from `next_tab_id`; closing the last tab returns focus to the sidebar. Table, query, files, dump and restore messages act on the active tab when it has the matching kind; `PageLoaded` and `DetailsLoaded` reach background tabs too.
- One script runs at a time (`running_script_tab_id`); its `ScriptFinished` is routed to the tab that started it, even after switching tabs, and that tab cannot be closed until it finishes. A pending save remembers its tab, so the saved name lands on the right document.
- The command palette (`Modal::Palette`) lists open tabs, actions (`Quit` last), `Switch to <label>` for every profile except the connected one, then the relations of every schema (current schema first). A non-empty query keeps labels containing its characters as a case-insensitive subsequence, ranked prefix (of the label or of the part after the last `.`), then substring, then subsequence; a query starting with `:` keeps only items whose `command_alias` starts with it (`:q` → `Quit`). Submitting performs the same transition as the matching message; opening a relation of another schema switches the sidebar to it.

- Connecting replaces the profile picker with a `Workspace` without tabs; reconnecting the same profile keeps the workspace and reloads the catalog and every table tab's page. Switching profile (`PaletteAction::SwitchProfile`) is refused while a script, a dump or a restore runs, sets `switching_profile_index` and emits `Connect`; its `Connected` swaps profile, session and tools and reloads the catalog and table tabs while query tabs keep their text and results, a failure only shows `Could not connect to <label>: <error>`, and a `Connected` for any other profile is ignored. `Run` records the profile label in `QueryView::ran_on_profile_label`. A catalog reload closes table tabs whose relation disappeared.
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
