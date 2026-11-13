# Key bindings

Source of truth: `apps/ducky-tui/src/keys/help.rs` (`HELP_ROWS`, shown with `F1` / `?`). Vim keys and arrows both work; in modals `↑`/`↓` work like `k`/`j`.

## Global

|Key|Action|
|---|---|
|`Ctrl+q`|quit|
|`q`|quit (outside the editor)|
|`F1 / ?`|help|
|`Tab / Shift+Tab`|switch focus|
|`Ctrl+c`|cancel the running script|

## Profiles

|Key|Action|
|---|---|
|`j / ↓`|next profile|
|`k / ↑`|previous profile|
|`Enter`|connect|

## Sidebar

|Key|Action|
|---|---|
|`j / ↓`|next item|
|`k / ↑`|previous item|
|`g / G`|first / last item|
|`Enter / l / →`|open the item|
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
|`g / G`|first / last row|
|`0 / $`|first / last column|
|`n / ]`|next page|
|`p / [`|previous page|
|`o`|cycle the sort|
|`/`|edit the WHERE filter|
|`Esc`|clear the filter, then sidebar|
|`Enter`|row detail|
|`y`|copy the cell|
|`r`|reload|
|`t`|structure tab|

## Table structure

|Key|Action|
|---|---|
|`j / k`|scroll|
|`g`|back to top|
|`t`|data tab|
|`r`|reload|
|`Esc`|sidebar|

## SQL editor

|Key|Action|
|---|---|
|`F5 / Ctrl+e`|run the script|
|`Ctrl+s`|save|
|`F2`|save as|
|`Ctrl+t`|toggle the transaction|
|`Ctrl+Space`|open completion|
|`Tab`|accept the completion|
|`↓ ↑ / Ctrl+n Ctrl+p`|move in the completion|
|`Esc`|close completion / normal mode|
|`i / a / o`|insert mode (vim)|

## Results

|Key|Action|
|---|---|
|`h j k l / arrows`|move the cursor|
|`g G 0 $`|first / last row / column|
|`[ / ]`|previous / next result|
|`Enter`|row detail|
|`y`|copy the cell|
|`Esc`|sidebar|

## SQL files

|Key|Action|
|---|---|
|`j / k`|select a file|
|`Enter`|open in the editor|
|`x`|execute|
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

## Help

|Key|Action|
|---|---|
|`Esc / q / ?`|close|

The editor otherwise follows edtui vim mode (`i`, `a`, `o`, `v`, `Esc`, motions, `u` undo, `p` paste). Prompts also accept `Backspace`, `Delete`, `←`/`→`, `Home`/`End`. Pasting into the editor in any mode inserts the text.
