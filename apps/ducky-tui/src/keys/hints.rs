use ducky_app::{
    AppState, CellInput, Focus, Modal, PickerMode, QueryPane, Screen, TabContent, TableTab,
};

use crate::keys::help::KeyHint;

const PICKER_FILTER_HINTS: &[KeyHint] = &[
    ("type", "filter"),
    ("↑↓", "move"),
    ("Enter", "connect"),
    ("Esc", "list"),
];
const PICKER_LIST_HINTS: &[KeyHint] = &[
    ("j/k", "move"),
    ("/", "filter"),
    ("Enter", "connect"),
    ("q", "quit"),
];
const SWITCH_FILTER_HINTS: &[KeyHint] = &[
    ("type", "filter"),
    ("↑↓", "move"),
    ("Enter", "switch"),
    ("Esc", "list"),
];
const SWITCH_LIST_HINTS: &[KeyHint] = &[
    ("j/k", "move"),
    ("/", "filter"),
    ("Enter", "switch"),
    ("Esc", "close"),
];
const SIDEBAR_HINTS: &[KeyHint] = &[
    ("Ctrl+p", "go to"),
    ("Enter", "open"),
    ("/", "filter"),
    ("s", "schema"),
    ("< >", "width"),
    ("?", "help"),
];
const NO_TAB_HINTS: &[KeyHint] = &[
    ("Ctrl+p", "go to"),
    ("Ctrl+n", "new query"),
    ("Ctrl+o", "SQL files"),
    ("Tab", "sidebar"),
    ("?", "help"),
];
const TABLE_DATA_HINTS: &[KeyHint] = &[
    ("hjkl", "move"),
    ("n/p", "page"),
    ("s", "sort"),
    ("/", "where"),
    ("v", "mark"),
    ("V", "visual"),
    ("e", "edit"),
    ("c", "change"),
    ("gd/gr", "references"),
    ("t", "structure"),
    ("?", "help"),
];
const TABLE_DRAFT_HINTS: &[KeyHint] = &[
    ("Ctrl+s", "save"),
    ("U", "discard"),
    ("x", "NULL"),
    ("u", "revert cell"),
    ("c", "change"),
    ("hjkl", "move"),
    ("v", "mark"),
    ("?", "help"),
];
const TABLE_VISUAL_HINTS: &[KeyHint] = &[
    ("j/k", "extend"),
    ("V", "keep"),
    ("e", "edit"),
    ("Esc", "clear"),
];
const TABLE_STRUCTURE_HINTS: &[KeyHint] = &[
    ("j/k", "scroll"),
    ("t", "data"),
    ("r", "reload"),
    ("Esc", "sidebar"),
];
const EDITOR_HINTS: &[KeyHint] = &[
    ("F5", "run"),
    ("Ctrl+s", "save"),
    ("F2", "save as"),
    ("Ctrl+t", "tx / autocommit"),
    ("Ctrl+Space", "complete"),
    ("Ctrl+p", "go to"),
];
const RESULTS_HINTS: &[KeyHint] = &[
    ("hjkl", "move"),
    ("p/n", "statement"),
    ("Enter", "row"),
    ("y", "copy"),
    ("F5", "run"),
    ("Ctrl+s", "save"),
    ("Esc", "sidebar"),
];
const FILES_HINTS: &[KeyHint] = &[
    ("j/k", "select"),
    ("Enter", "open"),
    ("x", "execute"),
    ("r", "reload"),
    ("Esc", "sidebar"),
];
const DUMP_HINTS: &[KeyHint] = &[
    ("Enter", "start"),
    ("e", "rename"),
    ("Ctrl+u/d", "log"),
    ("Esc", "sidebar"),
];
const RESTORE_HINTS: &[KeyHint] = &[
    ("j/k", "select"),
    ("c", "clean"),
    ("o", "owner"),
    ("Enter", "start"),
    ("r", "reload"),
    ("Esc", "sidebar"),
];

pub fn hints(state: &AppState) -> &'static [KeyHint] {
    if let Some(modal) = &state.modal {
        return modal_hints(modal);
    }
    let workspace = match &state.screen {
        Screen::Profiles(picker) => {
            return match picker.mode {
                PickerMode::Filter => PICKER_FILTER_HINTS,
                PickerMode::Navigate => PICKER_LIST_HINTS,
            };
        }
        Screen::Workspace(workspace) => workspace,
    };
    if workspace.focus == Focus::Sidebar {
        return SIDEBAR_HINTS;
    }
    match workspace.active_tab().map(|tab| &tab.content) {
        None => NO_TAB_HINTS,
        Some(TabContent::Table(table)) => match table.tab {
            TableTab::Data if table.visual_anchor_row_index.is_some() => TABLE_VISUAL_HINTS,
            TableTab::Data if !table.cell_drafts.is_empty() => TABLE_DRAFT_HINTS,
            TableTab::Data => TABLE_DATA_HINTS,
            TableTab::Structure => TABLE_STRUCTURE_HINTS,
        },
        Some(TabContent::Query(query)) => match query.pane {
            QueryPane::Editor => EDITOR_HINTS,
            QueryPane::Results => RESULTS_HINTS,
        },
        Some(TabContent::Files) => FILES_HINTS,
        Some(TabContent::Dump) => DUMP_HINTS,
        Some(TabContent::Restore) => RESTORE_HINTS,
    }
}

fn modal_hints(modal: &Modal) -> &'static [KeyHint] {
    match modal {
        Modal::Prompt(_) => &[
            ("Enter", "submit"),
            ("Esc", "cancel"),
            ("Ctrl+w", "delete word"),
            ("Ctrl+u", "clear"),
        ],
        Modal::Confirm(_) => &[("y/Enter", "confirm"), ("n/Esc", "cancel")],
        Modal::RowDetail(_) => &[("j/k", "move"), ("y", "copy value"), ("Esc", "close")],
        Modal::SchemaPicker { .. } => &[("j/k", "move"), ("Enter", "choose"), ("Esc", "cancel")],
        Modal::ReferencePicker { .. } => &[("Enter", "open"), ("j/k", "move"), ("Esc", "cancel")],
        Modal::Help => &[
            ("j/k", "scroll"),
            ("g", "top"),
            ("Ctrl+d/u", "half page"),
            ("Esc", "close"),
        ],
        Modal::Palette(_) => &[("Enter", "open"), ("↑↓", "move"), ("Esc", "close")],
        Modal::ProfilePicker(picker) => match picker.mode {
            PickerMode::Filter => SWITCH_FILTER_HINTS,
            PickerMode::Navigate => SWITCH_LIST_HINTS,
        },
        Modal::CellEditor(editor) => match editor.input {
            CellInput::Text(_) => &[("Enter", "set"), ("Esc", "cancel")],
            CellInput::Choice { .. } => &[("j/k", "move"), ("Enter", "set"), ("Esc", "cancel")],
        },
        Modal::Parameters(_) => &[
            ("Tab/S-Tab", "field"),
            ("Ctrl+r", "raw SQL"),
            ("Enter", "run"),
            ("Esc", "cancel"),
        ],
    }
}

pub fn mode_label(state: &AppState) -> Option<&'static str> {
    let workspace = state.workspace()?;
    if state.modal.is_some() || workspace.focus != Focus::Main {
        return None;
    }
    match &workspace.active_tab()?.content {
        TabContent::Table(table)
            if table.tab == TableTab::Data && table.visual_anchor_row_index.is_some() =>
        {
            Some("VISUAL")
        }
        _ => None,
    }
}
