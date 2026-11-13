# Architecture

## Crates

```
crates/ducky-core      domain types, no dependencies
crates/ducky-config    TOML profiles, lookup, ${VAR} expansion     -> core
crates/ducky-sql       highlight, statement split, completion       -> core
crates/ducky-postgres  sessions, introspection, pages, scripts      -> core, sql
crates/ducky-files     SQL file store, dump store, pg_dump/pg_restore -> core
crates/ducky-app       pure state machine (AppState, Msg, Command, update) -> core, sql
crates/ducky-runtime   executes Command on tokio, answers with Msg  -> app, config, sql, postgres, files
apps/ducky-tui         binary `ducky`: keys, rendering, effects     -> app, runtime, config, sql
```

Dependencies only point downwards. Nothing below `apps/ducky-tui` knows about the terminal.

## Event loop

`ducky-app::update(&mut AppState, Msg) -> Vec<Command>` is pure. `ducky-runtime::Runtime::dispatch(Command)` spawns the IO on tokio and sends answers back as `Msg` over an unbounded channel. The TUI maps keys to `Msg` and renders `AppState`.

```mermaid
sequenceDiagram
    participant User
    participant TUI as ducky-tui (keys / views)
    participant App as ducky-app::update
    participant RT as ducky-runtime
    participant PG as Postgres / files / pg tools
    User->>TUI: key press
    TUI->>App: Msg (e.g. Sidebar(Activate))
    App-->>TUI: Vec<Command> (e.g. LoadPage)
    TUI->>RT: dispatch(Command)
    RT->>PG: async work (tokio::spawn / spawn_blocking)
    PG-->>RT: result
    RT-->>TUI: Msg over channel (PageLoaded)
    TUI->>App: update(state, Msg)
    TUI->>User: draw(AppState)
```

Responses carry their request (`PageLoaded { request }`, `DetailsLoaded { relation }`, `FilePreviewLoaded { name }`, `ScriptFinished { tab_id, report }`); `update` drops responses that no longer match the state.

## Tabs and focus

`Workspace` holds `tabs: Vec<Tab>` and `active_tab_index`. A `Tab` has a `TabId` and a `TabContent`: `Table(TableView)` (one per relation from the sidebar or the palette, reused when reopened; foreign-key navigation adds one per relation and filter), `Query(QueryView)` (numbered `Query N` unless named after a file), or the singletons `Files`, `Dump`, `Restore`. Per-tab state (page, cursor, document, run state) lives in the tab; `TabsMsg` opens, switches and closes tabs. The sidebar lists only the relations of `schema_name`.

`Msg::OpenPalette` opens `Modal::Palette(CommandPalette)`; `Workspace::palette_items(profiles, query)` lists `PaletteItem`s whose `PaletteAction` switches to a tab, runs an action (new query, SQL files, dump, restore, schema, refresh, reconnect, disconnect, help, quit), switches to another profile or opens a relation of any schema. An item may carry a `command_alias` (`Quit` has `:q`): a query starting with `:` lists only the items whose alias starts with it. `Msg::FocusPane(PaneDirection)` moves focus between sidebar, main pane, editor and results.

## Switching profile

`PaletteAction::SwitchProfile(index)` keeps the workspace and its tabs and emits `Command::Connect` for the other profile, recording `Workspace::switching_profile_index`; it is refused while a script, a dump or a restore runs. `Msg::Connected { profile_index, .. }` in a workspace is a reconnect when `profile_index` is the workspace's, a switch when it matches `switching_profile_index` (success: profile, session and tools replaced, catalog and every table tab page reloaded, details reset; failure: error notice, the runtime keeps the old connection), and is ignored otherwise. `QueryView::ran_on_profile_label` records the profile a script was started on.

Only one script runs at a time: `running_script_tab_id` records the query tab that issued `Command::RunScript { tab_id, .. }`; `Run` from any tab is refused while it is set, and `ScriptFinished { tab_id, .. }` clears it and stores the report in that tab (if still open).

`QueryMsg::Run` (and `FileOpened { run_after_open: true }`) first applies the running / blank guards, then `ducky_sql::find_parameters`: without parameters it emits `RunScript`; otherwise it opens `Modal::Parameters(ParameterForm { tab_id, sql, fields, selected_field_index })`, one `ParameterField { name, input, is_raw }` per parameter prefilled from the tab's `QueryView::parameter_values`. `ModalMsg::Input` / `Next` / `Previous` / `ToggleRaw` edit the form; `Submit` merges the values into `parameter_values`, closes the form and runs `ducky_sql::bind_parameters(sql, values)` through the same guarded path, so reports hold the bound SQL.

`load_catalog` fills `Relation::foreign_keys` with `ForeignKeyLink { name, columns, referenced, referenced_columns }` (key order); `Catalog::referencing(relation)` derives the incoming links. `TableMsg::GoToReferenced` and `TableMsg::FindReferencing` (keys `gd` / `gr`) read the cursor row of the active data tab, build a filter with `ducky_sql::quote_identifier` / `quote_literal` (`"col" = 'value'` joined with ` AND `) and open a new `Table` tab on it, or activate a tab with the same relation and filter; several incoming links go through `Modal::ReferencePicker`. These tabs are plain filtered table tabs: the filter can be edited or cleared like any other.

Table pages use `AppState::page_size` (set with `AppState::with_page_size` from the config's `page_size`, default `DEFAULT_PAGE_SIZE`).

In `apps/ducky-tui`, `TuiState` keeps view-only state per tab in maps keyed by `TabId`: `editors` (one edtui `EditorPanel` per query tab), `table_viewports`, `results_viewports`, `structure_scrolls`.

## Adding a view

1. Add state to `ducky-app/src/state`, a `TabContent` variant (and a `PaletteAction` to open it), a `Msg` family and its `update/` module; add `Command`s if IO is needed, with tests in `crates/ducky-app/tests`.
2. Handle new commands in `ducky-runtime`, answering with `Msg`.
3. In `apps/ducky-tui`: `views/<name>.rs` (`render(frame, area, &ViewContext, &mut TuiState)`), `keys/<name>.rs`, dispatch in `views/mod.rs` and `keys/mod.rs`, rows in `keys/help.rs` and `docs/keybindings.md`.

## Another front-end

A Dioxus (or any) UI reuses every crate except `apps/ducky-tui`: own an `AppState`, feed events through `update`, pass commands to `Runtime::dispatch` (inside a tokio runtime), and render from state. `ducky-sql::highlight` gives tokens for colouring independent of the widget toolkit.

## Connections and transactions

Each session opens two connections with `tokio-postgres` (simple-query protocol, every value as text):

- `browse`: catalog, relation details, table pages.
- `script`: user SQL. A long or open script never blocks browsing; `Ctrl+c` sends a cancel request for it.

Scripts are split into statements by `ducky-sql::split_statements` and executed one by one, keeping at most 10 000 rows per result.

- `Transaction` (default, ` one transaction `): `BEGIN`, statements, `COMMIT`; the first failure stops and issues `ROLLBACK`.
- `AutoCommit` (` autocommit `, toggled with `Ctrl+t`): statements are sent as-is and stop at the first failure; earlier statements stay committed.

After a script that affected rows, the catalog is reloaded. Table data is read-only; writes go through the editor.
