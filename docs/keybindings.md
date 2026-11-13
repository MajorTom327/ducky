# Key bindings

Source of truth: `apps/ducky-tui/src/keys/help.rs` (`HELP_ROWS`, shown with `F1` / `?`). Vim keys and arrows both work; in modals `↑`/`↓` work like `k`/`j`.

## Global

|Key|Action|
|---|---|
|`Ctrl+p`|go to anything|
|`Ctrl+n`|new SQL query tab|
|`Ctrl+w`|close the active tab|
|`Ctrl+o`|open the SQL files tab|
|`Ctrl+h / Ctrl+l`|focus the sidebar / the main pane|
|`Ctrl+k / Ctrl+j`|focus the editor / the results|
|`Ctrl+PgDn`|next tab|
|`Ctrl+PgUp`|previous tab|
|`F1`|help|
|`Ctrl+c`|cancel the running script|
|`Ctrl+q`|quit|

## Outside insert mode

|Key|Action|
|---|---|
|`[b / ]b`|previous / next tab|
|`Space b d`|close the active tab|
|`Space q q`|quit|
|`:`|go to anything, starting with :|

## Outside the editor

|Key|Action|
|---|---|
|`1 … 9`|jump to tab N|
|`H / L`|previous / next tab|
|`Tab / Shift+Tab`|cycle the focus|
|`?`|help|
|`q`|quit|

## Go to anything

|Key|Action|
|---|---|
|`type`|filter tabs, actions, tables|
|`:q`|quit|
|`Switch to <profile>`|reconnect this workspace to another profile, tabs stay open|
|`↓ / Ctrl+n / Tab`|next item|
|`↑ / Ctrl+p`|previous item|
|`Enter`|open the item|
|`Ctrl+w / Ctrl+u`|delete a word / the query|
|`Esc`|close|

## Profiles

|Key|Action|
|---|---|
|`j / k`|next / previous profile|
|`Enter`|connect|

## Sidebar

|Key|Action|
|---|---|
|`j / k`|next / previous table|
|`g / G`|first / last table|
|`Enter / l / →`|open the table in a tab|
|`/`|filter tables|
|`Esc`|clear the filter|
|`s`|choose the schema|
|`r`|refresh the catalog|
|`R`|reconnect|
|`D`|disconnect|

## Table data

|Key|Action|
|---|---|
|`h j k l / arrows`|move the cursor|
|`gg / G`|first / last row|
|`0 / $`|first / last column|
|`n`|next page|
|`p`|previous page|
|`o`|cycle the sort of the column|
|`/`|edit the WHERE filter|
|`gd`|go to referenced row|
|`gr`|find referencing rows|
|`Esc`|clear the filter, then sidebar|
|`Enter`|row detail|
|`y`|copy the cell|
|`r`|reload|
|`t`|structure tab|

## Table structure

|Key|Action|
|---|---|
|`j / k`|scroll|
|`g`|back to the top|
|`t`|data tab|
|`r`|reload|
|`Esc`|sidebar|

## SQL editor

|Key|Action|
|---|---|
|`F5 / Ctrl+e`|run the script|
|`Ctrl+s`|save|
|`F2`|save as|
|`Ctrl+t`|one transaction / autocommit|
|`Ctrl+Space`|open the completion|
|`Tab`|accept the completion|
|`↓ ↑ / Ctrl+n p`|move in the completion|
|`Esc`|close completion / normal mode|
|`i / a / o`|insert mode (vim)|

## Results

|Key|Action|
|---|---|
|`h j k l / arrows`|move the cursor|
|`gg G 0 $`|first / last row or column|
|`p / n`|previous / next statement|
|`Enter`|row detail|
|`y`|copy the cell|
|`F5 / Ctrl+e`|run the editor content|
|`Ctrl+s / F2`|save / save as|
|`Ctrl+t`|one transaction / autocommit|
|`Esc`|sidebar|

## SQL files

|Key|Action|
|---|---|
|`j / k`|select a file|
|`Enter`|open in a query tab|
|`x`|execute the file|
|`r`|reload|
|`Ctrl+d / Ctrl+u`|scroll the preview|
|`Esc`|sidebar|

## Dump

|Key|Action|
|---|---|
|`e`|rename the dump file|
|`Enter`|start the dump|
|`Ctrl+u / Ctrl+d`|scroll the log|
|`Esc`|sidebar|

## Restore

|Key|Action|
|---|---|
|`j / k`|select a dump|
|`c`|toggle clean before restore|
|`o`|toggle skip ownership|
|`Enter`|start the restore|
|`r`|reload|
|`Ctrl+u / Ctrl+d`|scroll the log|
|`Esc`|sidebar|

## Prompt

|Key|Action|
|---|---|
|`Enter`|submit|
|`Esc`|cancel|
|`Ctrl+w`|delete the previous word|
|`Ctrl+u`|clear the line|

## Query parameters

|Key|Action|
|---|---|
|`type`|value of the selected parameter|
|`Tab / ↓`|next parameter|
|`Shift+Tab / ↑`|previous parameter|
|`Ctrl+r`|insert as raw SQL / as a string literal|
|`Ctrl+w / Ctrl+u`|delete a word / the value|
|`Enter`|run with these values|
|`Esc`|cancel|

## Confirm

|Key|Action|
|---|---|
|`y / Enter`|confirm|
|`n / Esc`|cancel|

## Row detail

|Key|Action|
|---|---|
|`j / k`|next / previous field|
|`y`|copy the value|
|`Esc / q`|close|

## Schema picker

|Key|Action|
|---|---|
|`j / k`|next / previous schema|
|`Enter`|choose|
|`Esc`|cancel|

## Reference picker

|Key|Action|
|---|---|
|`j / k`|next / previous foreign key|
|`Enter`|open the referencing rows|
|`Esc`|cancel|

## Help

|Key|Action|
|---|---|
|`j / k`|scroll|
|`g`|back to the top|
|`Ctrl+d / Ctrl+u`|half a page down / up|
|`Esc / q / ?`|close|

## Rules

- Global keys work in every pane, including the editor in insert mode.
- `[b`, `]b`, `Space b d` and `Space q q` are key sequences, accepted everywhere except in editor insert mode. The pending prefix is shown in the status bar; any other key cancels it.
- `gg`, `gd` and `gr` are `g` sequences of the table data tab (`gg` also in the results pane): `gd` (go to referenced row) opens the table referenced by the foreign key under the cursor in a new tab filtered on the referenced key, `gr` (find referencing rows) opens the rows of another table whose foreign key points at the current row, through a picker when several foreign keys do. Foreign-key columns end with a muted ` →` in the grid header.
- `:` opens go to anything with `:` already typed, everywhere except in editor insert mode (where it types `:`). A query starting with `:` lists only the items that have a command alias (`:q` quits); other queries match `Quit` by its label.
- Go to anything lists `Switch to <profile>` for every profile except the connected one, with the profile type in its colour. Picking one reconnects the workspace: tabs, query text and results stay open, the catalog and every table tab reload from the new profile, and the results title shows the profile a script ran on (` 12 ms · one transaction · on Dev local `). Switching is refused while a script, a dump or a restore runs; if the connection fails, the current profile stays connected.
- `1`–`9`, `H` / `L`, `Tab`, `?` and `q` apply outside the SQL editor.
- `Ctrl+h` focuses the sidebar, `Ctrl+l` the main pane, `Ctrl+k` the SQL editor and `Ctrl+j` the results of the active query tab.
- The editor follows edtui vim mode (`i`, `a`, `o`, `v`, `Esc`, motions, `u` undo, `p` paste). Its bottom border shows the vim mode on the left and ` one transaction · Ctrl+t ` or ` autocommit · Ctrl+t ` on the right. The terminal cursor is a bar in insert mode and a block otherwise.
- Prompts and the query parameters form also accept `Backspace`, `Delete`, `←`/`→`, `Home`/`End`. Pasting into the editor in any mode inserts the text.
- The help popup uses two columns only when the terminal is wide enough to show every row untruncated.
