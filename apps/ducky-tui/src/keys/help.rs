use ducky_app::{AppState, Focus, MainView, Modal, QueryPane, Screen, TableTab};

pub const HELP_ROWS: &[(&str, &str)] = &[
    ("", "Global"),
    ("Ctrl+q", "quit"),
    ("q", "quit (outside the editor)"),
    ("F1 / ?", "help"),
    ("Tab / Shift+Tab", "switch focus"),
    ("Ctrl+c", "cancel the running script"),
    ("", "Profiles"),
    ("j / ↓", "next profile"),
    ("k / ↑", "previous profile"),
    ("Enter", "connect"),
    ("", "Sidebar"),
    ("j / ↓", "next item"),
    ("k / ↑", "previous item"),
    ("g / G", "first / last item"),
    ("Enter / l / →", "open the item"),
    ("/", "filter tables"),
    ("Esc", "clear the filter"),
    ("s", "choose the schema"),
    ("r", "refresh the catalog"),
    ("R", "reconnect"),
    ("D", "disconnect"),
    ("", "Table data"),
    ("h j k l / arrows", "move the cursor"),
    ("g / G", "first / last row"),
    ("0 / $", "first / last column"),
    ("n / ]", "next page"),
    ("p / [", "previous page"),
    ("o", "cycle the sort"),
    ("/", "edit the WHERE filter"),
    ("Esc", "clear the filter, then sidebar"),
    ("Enter", "row detail"),
    ("y", "copy the cell"),
    ("r", "reload"),
    ("t", "structure tab"),
    ("", "Table structure"),
    ("j / k", "scroll"),
    ("g", "back to top"),
    ("t", "data tab"),
    ("r", "reload"),
    ("Esc", "sidebar"),
    ("", "SQL editor"),
    ("F5 / Ctrl+e", "run the script"),
    ("Ctrl+s", "save"),
    ("F2", "save as"),
    ("Ctrl+t", "toggle the transaction"),
    ("Ctrl+Space", "open completion"),
    ("Tab", "accept the completion"),
    ("↓ ↑ / Ctrl+n Ctrl+p", "move in the completion"),
    ("Esc", "close completion / normal mode"),
    ("i / a / o", "insert mode (vim)"),
    ("", "Results"),
    ("h j k l / arrows", "move the cursor"),
    ("g G 0 $", "first / last row / column"),
    ("[ / ]", "previous / next result"),
    ("Enter", "row detail"),
    ("y", "copy the cell"),
    ("Esc", "sidebar"),
    ("", "SQL files"),
    ("j / k", "select a file"),
    ("Enter", "open in the editor"),
    ("x", "execute"),
    ("r", "reload"),
    ("Ctrl+d / Ctrl+u", "scroll the preview"),
    ("Esc", "sidebar"),
    ("", "Dump"),
    ("e", "rename the dump file"),
    ("Enter", "start the dump"),
    ("Ctrl+u / Ctrl+d", "scroll the log"),
    ("Esc", "sidebar"),
    ("", "Restore"),
    ("j / k", "select a dump"),
    ("c", "toggle clean before restore"),
    ("o", "toggle skip ownership"),
    ("Enter", "start the restore"),
    ("r", "reload"),
    ("Ctrl+u / Ctrl+d", "scroll the log"),
    ("Esc", "sidebar"),
    ("", "Prompt"),
    ("Enter", "submit"),
    ("Esc", "cancel"),
    ("Ctrl+w", "delete the previous word"),
    ("Ctrl+u", "clear the line"),
    ("", "Confirm"),
    ("y / Enter", "confirm"),
    ("n / Esc", "cancel"),
    ("", "Row detail"),
    ("j / k", "next / previous field"),
    ("y", "copy the value"),
    ("Esc / q", "close"),
    ("", "Schema picker"),
    ("j / k", "next / previous schema"),
    ("Enter", "choose"),
    ("Esc", "cancel"),
    ("", "Help"),
    ("Esc / q / ?", "close"),
];

pub fn hints(state: &AppState) -> &'static str {
    if let Some(modal) = &state.modal {
        return match modal {
            Modal::Prompt(_) => "Enter submit · Esc cancel · Ctrl+w delete word · Ctrl+u clear",
            Modal::Confirm(_) => "y / Enter confirm · n / Esc cancel",
            Modal::RowDetail(_) => "j/k move · y copy value · Esc close",
            Modal::SchemaPicker { .. } => "j/k move · Enter choose · Esc cancel",
            Modal::Help => "Esc / q / ? close",
        };
    }
    let workspace = match &state.screen {
        Screen::Profiles(_) => return "j/k select · Enter connect · q quit",
        Screen::Workspace(workspace) => workspace,
    };
    if workspace.focus == Focus::Sidebar {
        return "j/k move · Enter open · / filter · s schema · r refresh · Tab focus · ? help";
    }
    match workspace.view {
        MainView::Welcome => "Tab sidebar · ? help · q quit",
        MainView::Table => match workspace.table.as_ref().map(|table| table.tab) {
            Some(TableTab::Structure) => "j/k scroll · t data · r reload · Esc sidebar",
            Some(TableTab::Data) | None => {
                "hjkl move · n/p page · o sort · / where · Enter row · y copy · t structure"
            }
        },
        MainView::Query => match workspace.query.pane {
            QueryPane::Editor => {
                "F5 run · Ctrl+s save · F2 save as · Ctrl+t transaction · Ctrl+Space complete"
            }
            QueryPane::Results => "hjkl move · [ ] result · Enter row · y copy · Esc sidebar",
        },
        MainView::Files => "j/k select · Enter open · x execute · r reload · Esc sidebar",
        MainView::Dump => "Enter start · e rename · Ctrl+u/Ctrl+d log · Esc sidebar",
        MainView::Restore => {
            "j/k select · c clean · o owner · Enter start · r reload · Esc sidebar"
        }
    }
}
