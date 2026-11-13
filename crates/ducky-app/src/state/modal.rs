use ducky_core::{Cell, ForeignKeyLink, RelationRef};
use tui_input::Input;

use crate::state::palette::CommandPalette;
use crate::state::parameters::ParameterForm;
use crate::state::profiles::ProfilePicker;
use crate::state::tabs::TabId;

#[derive(Debug, Clone)]
pub enum Modal {
    Prompt(Prompt),
    Confirm(Confirm),
    RowDetail(RowDetail),
    SchemaPicker {
        selected_schema_index: usize,
    },
    Help,
    Palette(CommandPalette),
    Parameters(ParameterForm),
    ProfilePicker(ProfilePicker),
    ReferencePicker {
        links: Vec<(RelationRef, ForeignKeyLink)>,
        selected_link_index: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptKind {
    SidebarFilter,
    TableFilter,
    SaveFileAs { content: String },
    DumpFileName,
}

#[derive(Debug, Clone)]
pub struct Prompt {
    pub kind: PromptKind,
    pub title: String,
    pub input: Input,
}

impl Prompt {
    pub(crate) fn new(kind: PromptKind, title: &str, initial_value: String) -> Prompt {
        Prompt {
            kind,
            title: title.to_owned(),
            input: Input::new(initial_value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmKind {
    OverwriteFile,
    RunScript { tab_id: TabId, sql: String },
    Restore { file_name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Confirm {
    pub kind: ConfirmKind,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowDetail {
    pub columns: Vec<String>,
    pub values: Vec<Cell>,
    pub selected_field_index: usize,
}
