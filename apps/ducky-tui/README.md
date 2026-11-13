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
- `keys::handle_key(state, tui, key)` and `keys::handle_paste(state, tui, text)`; `keys::help::{HELP_ROWS, hints}`; `keys::sequence` handles the `[b` / `]b` / `Space b d` sequences and their status-bar hints.
- `theme::{Palette, palette, token_style}`: accent colour per profile kind (`local` green, `staging` orange, `prod` red, cyan before connecting).
- `layout::FrameAreas`: header, sidebar (32 columns), main area (tab bar and active tab) and status bar. Below 80x20 only a size warning is drawn.
- `widgets::grid::{GridViewport, render_grid}`, `widgets::loading::{LoadSlot, render_loading, render_message, render_error, render_loadable}`, `widgets::sql_text::highlighted_sql`.
- `editor::panel::EditorPanel`: the edtui vim editor bound to one query tab's document; its bottom border shows the vim mode and the transaction mode.
- `effects::UiEffects`: startup coalesce, loading pulse per slot, workspace fade-in, modal and notice effects.

## Tests

```sh
cargo test -p ducky-tui
```

`tests/render.rs` renders fabricated states on a 120x36 `TestBackend`: header colours per profile kind, profile list, sidebar content, missing tools, grid rendering, the size warning, the loading pulse lifecycle and key precedence.
