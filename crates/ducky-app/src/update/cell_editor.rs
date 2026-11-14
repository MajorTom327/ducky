use ducky_core::{Cell, Column, RelationKind};
use tui_input::Input;

use crate::command::Command;
use crate::state::{
    AppState, CellEditor, CellInput, GridCursor, Modal, Notice, NoticeLevel, TabContent, TableTab,
    TableView, Workspace,
};
use crate::update::cell_edits::sql_expression;
use crate::update::row_keys::{row_keys, target_row_indexes};
use crate::update::workspace_mut;

const BOOLEAN_TYPE: &str = "boolean";
const BOOLEAN_TRUE: &str = "t";
const BOOLEAN_FALSE: &str = "f";
const GEO_TYPE_PREFIXES: [&str; 2] = ["geometry", "geography"];

struct EditTarget<'a> {
    column: &'a Column,
    column_name: &'a String,
    column_index: usize,
    row_indexes: Vec<usize>,
    current_value: &'a Cell,
}

enum CellChange {
    Toggle {
        row_indexes: Vec<usize>,
        column_index: usize,
        draft_value: Cell,
    },
    Edit(CellEditor),
}

pub(crate) fn change_cell(state: &mut AppState) -> Vec<Command> {
    let cell_change = match state.workspace().and_then(cell_change) {
        None => return Vec::new(),
        Some(Err(refusal)) => {
            state.notice = Some(refusal);
            return Vec::new();
        }
        Some(Ok(cell_change)) => cell_change,
    };
    match cell_change {
        CellChange::Toggle {
            row_indexes,
            column_index,
            draft_value,
        } => {
            if let Some(table) =
                workspace_mut(&mut state.screen).and_then(Workspace::active_table_mut)
            {
                commit(table, column_index, &row_indexes, draft_value);
            }
        }
        CellChange::Edit(editor) => state.modal = Some(Modal::CellEditor(editor)),
    }
    Vec::new()
}

pub(crate) fn set_cell_null(state: &mut AppState) -> Vec<Command> {
    let null_change = state.workspace().and_then(edit_target).map(|target| {
        let target = target?;
        match target.column.is_nullable {
            true => Ok((target.column_index, target.row_indexes)),
            false => Err(Notice::new(
                NoticeLevel::Warning,
                format!("{} is NOT NULL", target.column_name),
            )),
        }
    });
    match null_change {
        None => {}
        Some(Err(refusal)) => state.notice = Some(refusal),
        Some(Ok((column_index, row_indexes))) => {
            if let Some(table) =
                workspace_mut(&mut state.screen).and_then(Workspace::active_table_mut)
            {
                commit(table, column_index, &row_indexes, None);
            }
        }
    }
    Vec::new()
}

pub(crate) fn submit(state: &mut AppState, editor: CellEditor) -> Vec<Command> {
    let draft_value = match editor.input {
        CellInput::Text(input) => text_draft(input.value()),
        CellInput::Choice {
            options,
            selected_option_index,
        } => {
            let Some(selected_option) = options.into_iter().nth(selected_option_index) else {
                return Vec::new();
            };
            selected_option
        }
    };
    if let Some(table) = workspace_mut(&mut state.screen).and_then(Workspace::active_table_mut) {
        commit(table, editor.column_index, &editor.row_indexes, draft_value);
    }
    Vec::new()
}

fn text_draft(text: &str) -> Cell {
    match sql_expression(text) {
        Some(expression) if expression.trim().eq_ignore_ascii_case("null") => None,
        _ => Some(text.to_owned()),
    }
}

fn edit_target(workspace: &Workspace) -> Option<Result<EditTarget<'_>, Notice>> {
    let TabContent::Table(table) = &workspace.active_tab()?.content else {
        return None;
    };
    if table.tab != TableTab::Data || table.is_saving_drafts {
        return None;
    }
    let result_set = &table.page.ready()?.result_set;
    let column_index = table.cursor.column_index;
    let column_name = result_set.columns.get(column_index)?;
    let original_value = result_set
        .rows
        .get(table.cursor.row_index)?
        .get(column_index)?;
    let relation = workspace.catalog.ready()?.relation(&table.relation)?;
    if matches!(
        relation.kind,
        RelationKind::View | RelationKind::MaterializedView
    ) {
        return Some(Err(Notice::new(
            NoticeLevel::Warning,
            "Views are read-only",
        )));
    }
    let column = relation
        .columns
        .iter()
        .find(|column| column.name == *column_name)?;
    let row_indexes = target_row_indexes(table, result_set);
    if let Err(message) = row_keys(workspace, table, result_set, &row_indexes) {
        return Some(Err(Notice::new(NoticeLevel::Error, message)));
    }
    Some(Ok(EditTarget {
        column,
        column_name,
        column_index,
        row_indexes,
        current_value: table
            .cell_drafts
            .get(&table.cursor)
            .unwrap_or(original_value),
    }))
}

fn cell_change(workspace: &Workspace) -> Option<Result<CellChange, Notice>> {
    let target = match edit_target(workspace)? {
        Ok(target) => target,
        Err(refusal) => return Some(Err(refusal)),
    };
    let column = target.column;
    if GEO_TYPE_PREFIXES
        .iter()
        .any(|geo_type_prefix| column.data_type.starts_with(geo_type_prefix))
    {
        return Some(Err(Notice::new(
            NoticeLevel::Warning,
            "Geometry columns cannot be edited inline",
        )));
    }
    if column.enum_labels.is_empty() && column.data_type == BOOLEAN_TYPE {
        return Some(Ok(CellChange::Toggle {
            row_indexes: target.row_indexes,
            column_index: target.column_index,
            draft_value: next_boolean(target.current_value, column.is_nullable),
        }));
    }
    let input = match column.enum_labels.is_empty() {
        true => CellInput::Text(Input::new(target.current_value.clone().unwrap_or_default())),
        false => choice_input(column, target.current_value),
    };
    Some(Ok(CellChange::Edit(CellEditor {
        column_name: target.column_name.clone(),
        column_index: target.column_index,
        row_indexes: target.row_indexes,
        input,
    })))
}

fn next_boolean(current_value: &Cell, is_nullable: bool) -> Cell {
    let next_text = match current_value.as_deref() {
        Some(BOOLEAN_TRUE) => BOOLEAN_FALSE,
        Some(BOOLEAN_FALSE) if is_nullable => return None,
        _ => BOOLEAN_TRUE,
    };
    Some(next_text.to_owned())
}

fn choice_input(column: &Column, current_value: &Cell) -> CellInput {
    let mut options: Vec<Cell> = column.enum_labels.iter().cloned().map(Some).collect();
    if column.is_nullable {
        options.push(None);
    }
    let selected_option_index = options
        .iter()
        .position(|option| option == current_value)
        .unwrap_or(0);
    CellInput::Choice {
        options,
        selected_option_index,
    }
}

fn commit(table: &mut TableView, column_index: usize, row_indexes: &[usize], draft_value: Cell) {
    let Some(page) = table.page.ready() else {
        return;
    };
    for row_index in row_indexes {
        let draft_cursor = GridCursor {
            row_index: *row_index,
            column_index,
        };
        let is_back_to_original = page
            .result_set
            .rows
            .get(*row_index)
            .and_then(|row| row.get(column_index))
            .is_some_and(|original_value| is_unchanged(original_value, &draft_value));
        match is_back_to_original {
            true => table.cell_drafts.remove(&draft_cursor),
            false => table.cell_drafts.insert(draft_cursor, draft_value.clone()),
        };
    }
    table.clear_selection();
}

fn is_unchanged(original_value: &Cell, draft_value: &Cell) -> bool {
    original_value == draft_value
        || (original_value.is_none() && draft_value.as_deref() == Some(""))
}
