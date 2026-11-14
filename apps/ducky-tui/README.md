# ducky-tui

Terminal front-end of ducky: the `ducky` binary. It maps key presses to `ducky_app::Msg`, renders `ducky_app::AppState` with Ratatui and hands every `ducky_app::Command` to `ducky_runtime::Runtime`. Business logic lives in the library crates; this crate only owns keys, layout, rendering and effects.

## Running

```sh
cargo run -p ducky-tui -- --config dev/ducky.dev.toml
just dev
```

Arguments:

- `--config PATH`: configuration file (default lookup: `./ducky.toml`, then `$XDG_CONFIG_HOME/ducky/config.toml` or `$HOME/.config/ducky/config.toml`).
- `--profile LABEL` or `--profile PROJECT/LABEL`: connect to that profile on start. An unknown profile lists the profiles grouped by project and exits with code 2.
- `--no-effects`: disable tachyonfx animations.

A configuration error prints the error, the searched paths and an example configuration to stderr, then exits with code 1 without touching the terminal.

## Public API (library `ducky_tui`)

- `cli::Args`: command-line arguments.
- `event_loop::run(terminal, state, tui, runtime, messages, startup_messages)`: the `Msg → update → Command → Runtime` loop; key presses go to `keys::handle_key`, pastes to `keys::handle_paste`, mouse events to `mouse::handle_mouse`. `main.rs` enables mouse capture next to bracketed paste and disables both on exit.
- `tui_state::TuiState`: view-only state: effects, one `EditorPanel` per query tab and grid viewports / structure scroll / on-screen cursor cell (`table_cursor_cells`, the cell editor's anchor) per tab (maps keyed by `TabId`), preview, log and help scroll, cursor shape (bar in insert mode, block otherwise), the pending key sequence, and the mouse state: `mouse_targets` (hit areas of the last frame), `is_dragging_sidebar`, `last_click` (target and time, for double clicks).
- `mouse::handle_mouse(state, tui, event) -> Vec<Msg>`: hit-tests the event against `TuiState::mouse_targets` (the last `MouseTarget` containing the point wins, so modals beat the UI below) and maps left clicks, double clicks (same target within `DOUBLE_CLICK_INTERVAL`, 400 ms), sidebar border drags and the wheel (`WHEEL_LINE_COUNT`, 3) to messages; editor clicks, drags and wheel go to edtui. Any press dismisses the notice first; right and middle buttons are ignored. `MouseTarget` and `ScrollArea` name the hit areas; `list_row_targets` builds one target per visible list row.
- `views::draw(frame, state, tui)` and `views::ViewContext`; one module per view.
- `profile_lookup::resolve_profile(profiles, query) -> Result<usize, ProfileLookupError>`: `--profile` resolution by `LABEL`, `PROJECT/LABEL` or `PROJECT/TYPE` (`NotFound`, `Ambiguous { matching_profile_indexes }`).
- `keys::handle_key(state, tui, key)` and `keys::handle_paste(state, tui, text)`; `keys::help::HELP_ROWS`, `keys::hints::hints` (status-bar hints per context and modal); `keys::sequence` handles the `[b` / `]b` / `Space b d` / `Space e` (toggle the sidebar) / `Space p` (profile picker) / `Space q q` sequences, the `g` prefix of grid panes (`gg` first row in table data and results, `gd` / `gr` foreign-key navigation in table data) and their status-bar hints (the results pane only hints `g first row`); `keys::hints::mode_label` returns `VISUAL` while a table row range is open, shown before the visual hints; `keys::table` maps `v` / `V` / `e` to row marks, the visual range and the UPDATE preset, and `Esc` clears a selection before the filter, and `:` (outside insert mode), which opens the palette with `:` typed; `keys::sidebar` maps `<` / `>` to narrowing / widening the sidebar; `keys::jobs::scroll_job_log` scrolls the dump / restore log for keys and the wheel.
- `views::palette::{render, action_shortcut}`: the go to anything popup; the shortcut column shows an item's command alias (`:q`) or its key, `Switch profile…` shows `Space p` and `Toggle sidebar` shows `Space e`.
- `views::profile_picker::{render, fitted_area, content_height, PickerView}`: the profile picker shared by the start screen (`views::profiles::render`, ` Connect `, rows are `MouseTarget::ProfileRow`) and the switch popup (`views::profiles::render_switch_popup`, ` Switch profile `, rows are `MouseTarget::ModalItem`): filter line (`type to filter` placeholder, cursor in filter mode), project headers with their profile count, profile rows (kind badge in the kind accent, label, muted `#tags`, right-aligned muted target, `●` on the connected profile, selection in the selected profile's accent; drawn by `views::profile_rows`), `n of m` count, `Connecting…` and the connection error.
- `keys::profiles::handle(picker, key, is_modal)`: picker keys for both screens (filter mode edits the filter, navigate mode uses vim keys).
- `views::reference_picker::{render, link_label}`: the find referencing rows picker (`orders.customer_id → customers.id` per foreign key, footer `Enter open · j/k move · Esc cancel`).
- `views::confirm::render`: the confirmation popup; a run confirmation adds the first three SQL lines (`…` when longer) and `Mode: one transaction | autocommit` in muted text and frames the popup in the connected profile's accent; a cell-edit save confirmation shows the same SQL preview and accent without the mode line.
- `views::cell_editor::render(frame, screen, editor, anchor, palette, mouse_targets)`: the cell editor drawn over the cursor cell: a one-line text input (at least 24 columns wide, terminal cursor inside) or a bordered choice list under the cell (above when there is no room, at most 8 rows, `NULL` muted, one `ModalItem` target per visible option); centered without an anchor.
- `views::help::render`: the help popup (one or two columns, scrolled by `TuiState::help_scroll`).
- `views::parameters::render`: the query parameters form (name, input, `→` preview of the inserted SQL, `raw` badge, key footer).
- `theme::{Palette, palette, token_style}`: accent colour per profile kind (`local` green, `staging` orange, `prod` red, cyan before connecting).
- `layout::FrameAreas::new(screen, sidebar_width)`: header, sidebar, main area (tab bar and active tab) and status bar; `layout::sidebar_width(sidebar, screen_width)` is 0 for a collapsed sidebar, else its width capped so the main column keeps `MIN_MAIN_WIDTH` (40) columns. The profile picker screen ignores the sidebar (`FrameAreas::body`). Below 80x20 only a size warning is drawn.
- `widgets::grid::{GridOptions, GridViewport, render_grid}` (`GridOptions::linked_columns` get a muted ` →` header suffix; rows in the sorted `GridOptions::selected_row_indexes` get a `▌` gutter marker and muted bold text; `GridOptions::draft_cells` replace page values and show in bold yellow except under the cursor; `render_grid` returns the cursor cell's area and pushes `Grid`, `GridHeader` and `GridCell` targets tagged with `GridOptions::pane`), `widgets::loading::{LoadSlot, render_loading, render_message, render_error, render_loadable}`, `widgets::sql_text::highlighted_sql`.
- `editor::panel::EditorPanel`: the edtui vim editor bound to one query tab's document; its bottom border shows the vim mode and the transaction mode.
- `effects::UiEffects`: startup coalesce, loading pulse per slot, workspace fade-in, modal and notice effects.

## Tests

```sh
cargo test -p ducky-tui
```

`tests/run_confirmation.rs` renders the run confirmation (message, SQL preview, mode, prod accent border) and checks its `y` / `Enter` / `n` / `Esc` keys. `tests/table_view.rs` also covers the selection keys, the `▌` marker of selected rows and the ` · <n> selected` border count. `tests/render.rs` renders fabricated states on a 120x36 `TestBackend`: header colours per profile kind, profile picker, sidebar content, missing tools, grid rendering, the size warning, the loading pulse lifecycle and key precedence. `tests/profile_picker.rs` covers the picker rendering (project headers, badges, tags, targets, `n of m` count, `●` on the connected profile in the switch popup), the filter and navigate keys, `Space p` and the project in the header; `tests/chrome.rs` covers tabs, the palette (one `Switch profile…` item and the `:q` hint), help and key sequences (`Space q q`, `:`); `tests/foreign_keys.rs` covers the `g` prefix (`gg`, `gd`, `gr`, cancel, results-only hints), the `VISUAL` status hints, the ` →` header of foreign-key columns and the reference picker; `tests/parameters_form.rs` covers the query parameters form and its keys; `tests/query_view.rs` covers the editor and results (including ` · on <profile> `); `tests/workspace_flow.rs` (ignored, needs `DUCKY_TEST_DATABASE_URL`) drives a full session including a profile switch, `follow_foreign_keys_between_tabs` follows `gd` then `gr` across filtered tabs, and `edit_selected_rows_opens_an_update_preset` marks two rows with `v` and opens their UPDATE preset with `e`.

`tests/sidebar_layout.rs` covers the layout with a collapsed, wide and capped sidebar, `Space e`, `<` / `>`, the sidebar hints and the `Toggle sidebar` palette shortcut. `tests/mouse.rs` and `tests/mouse_lists.rs` render states on a `TestBackend`, then feed `MouseEvent`s at coordinates found in the buffer (fixtures in `tests/mouse_support/mod.rs`): sidebar clicks and wheel, tab clicks, grid cell clicks and double clicks, header clicks, the grid wheel, results and editor focus, the sidebar border drag, the structure / help / preview / log wheel, palette, schema and profile popup items, clicks inside and outside a modal, start-screen profiles, SQL files and dumps. `tests/event_loop.rs` checks that a scripted `Event::Mouse` reaches `update`.

`tests/cell_edits.rs` covers draft cell edits: drafted values in bold yellow in the grid and the cursor cell area returned by `render_grid`, the text editor drawn over the cursor cell with the terminal cursor, the enum dropdown (`NULL` muted, under the cell, above near the bottom, centered without an anchor), the ` N unsaved ` / ` saving… ` summary, the draft hints, the save confirmation preview, and the `c` / `u` / `U` / `Ctrl+s` and cell editor keys.
