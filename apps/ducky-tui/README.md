# ducky-tui

Terminal front-end of ducky: the `ducky` binary. It maps key presses to `ducky_app::Msg`, renders `ducky_app::AppState` with Ratatui and hands every `ducky_app::Command` to `ducky_runtime::Runtime`. Business logic lives in the library crates; this crate only owns keys, layout, rendering and effects.

## Running

```sh
cargo run -p ducky-tui -- --config dev/ducky.dev.toml
just dev
```

Arguments:

- `--config PATH`: configuration file (default lookup: `./ducky.toml`, then `$XDG_CONFIG_HOME/ducky/config.toml` or `$HOME/.config/ducky/config.toml`).
- `--profile LABEL`: connect to that profile on start. An unknown label exits with code 2.
- `--no-effects`: disable tachyonfx animations.

A configuration error prints the error, the searched paths and an example configuration to stderr, then exits with code 1 without touching the terminal.

## Public API (library `ducky_tui`)

- `cli::Args`: command-line arguments.
- `event_loop::run(terminal, state, tui, runtime, messages, startup_messages)`: the `Msg → update → Command → Runtime` loop.
- `tui_state::TuiState`: view-only state: effects, one `EditorPanel` per query tab and grid viewports / structure scroll per tab (maps keyed by `TabId`), preview, log and help scroll, cursor shape (bar in insert mode, block otherwise) and the pending key sequence.
- `views::draw(frame, state, tui)` and `views::ViewContext`; one module per view.
- `keys::handle_key(state, tui, key)` and `keys::handle_paste(state, tui, text)`; `keys::help::HELP_ROWS`, `keys::hints::hints` (status-bar hints per context and modal); `keys::sequence` handles the `[b` / `]b` / `Space b d` / `Space q q` sequences, the `g` prefix of grid panes (`gg` first row in table data and results, `gd` / `gr` foreign-key navigation in table data) and their status-bar hints (the results pane only hints `g first row`); `keys::hints::mode_label` returns `VISUAL` while a table row range is open, shown before the visual hints; `keys::table` maps `v` / `V` / `e` to row marks, the visual range and the UPDATE preset, and `Esc` clears a selection before the filter, and `:` (outside insert mode), which opens the palette with `:` typed.
- `views::palette::{render, action_shortcut}`: the go to anything popup; the shortcut column shows an item's command alias (`:q`) or its key, and `Switch to <profile>` details use that profile's accent.
- `views::reference_picker::{render, link_label}`: the find referencing rows picker (`orders.customer_id → customers.id` per foreign key, footer `Enter open · j/k move · Esc cancel`).
- `views::parameters::render`: the query parameters form (name, input, `→` preview of the inserted SQL, `raw` badge, key footer).
- `theme::{Palette, palette, token_style}`: accent colour per profile kind (`local` green, `staging` orange, `prod` red, cyan before connecting).
- `layout::FrameAreas`: header, sidebar (32 columns), main area (tab bar and active tab) and status bar. Below 80x20 only a size warning is drawn.
- `widgets::grid::{GridOptions, GridViewport, render_grid}` (`GridOptions::linked_columns` get a muted ` →` header suffix; rows in the sorted `GridOptions::selected_row_indexes` get a `▌` gutter marker and muted bold text), `widgets::loading::{LoadSlot, render_loading, render_message, render_error, render_loadable}`, `widgets::sql_text::highlighted_sql`.
- `editor::panel::EditorPanel`: the edtui vim editor bound to one query tab's document; its bottom border shows the vim mode and the transaction mode.
- `effects::UiEffects`: startup coalesce, loading pulse per slot, workspace fade-in, modal and notice effects.

## Tests

```sh
cargo test -p ducky-tui
```

`tests/table_view.rs` also covers the selection keys, the `▌` marker of selected rows and the ` · <n> selected` border count. `tests/render.rs` renders fabricated states on a 120x36 `TestBackend`: header colours per profile kind, profile list, sidebar content, missing tools, grid rendering, the size warning, the loading pulse lifecycle and key precedence. `tests/chrome.rs` covers tabs, the palette (including `Switch to <profile>` and the `:q` hint), help and key sequences (`Space q q`, `:`); `tests/foreign_keys.rs` covers the `g` prefix (`gg`, `gd`, `gr`, cancel, results-only hints), the `VISUAL` status hints, the ` →` header of foreign-key columns and the reference picker; `tests/parameters_form.rs` covers the query parameters form and its keys; `tests/query_view.rs` covers the editor and results (including ` · on <profile> `); `tests/workspace_flow.rs` (ignored, needs `DUCKY_TEST_DATABASE_URL`) drives a full session including a profile switch, `follow_foreign_keys_between_tabs` follows `gd` then `gr` across filtered tabs, and `edit_selected_rows_opens_an_update_preset` marks two rows with `v` and opens their UPDATE preset with `e`.
