use ducky_core::{Cell, ResultSet};
use ducky_sql::{quote_identifier, quote_identifier_if_needed, quote_literal};

use crate::command::Command;
use crate::msg::GridMove;
use crate::state::{
    AppState, Document, Focus, NoticeLevel, TabContent, TableTab, TableView, Workspace,
};
use crate::update::grid::move_cursor;
use crate::update::{notice, workspace_mut};

const FALLBACK_KEY_COLUMN: &str = "id";
const FALLBACK_PARAMETER_NAME: &str = "value";

struct UpdatePreset {
    text: String,
    row_count: usize,
    parameter_name: String,
}

pub(crate) fn toggle_row_mark(table: &mut TableView) -> Vec<Command> {
    let Some(page) = table.page.ready() else {
        return Vec::new();
    };
    if page.result_set.rows.is_empty() {
        return Vec::new();
    }
    let cursor_row_index = table.cursor.row_index;
    if !table.marked_row_indexes.remove(&cursor_row_index) {
        table.marked_row_indexes.insert(cursor_row_index);
    }
    move_cursor(&mut table.cursor, &page.result_set, GridMove::Down);
    Vec::new()
}

pub(crate) fn toggle_visual_selection(table: &mut TableView) -> Vec<Command> {
    if table.visual_anchor_row_index.is_some() {
        let selected_row_indexes = table.selected_row_indexes();
        table.marked_row_indexes.extend(selected_row_indexes);
        table.visual_anchor_row_index = None;
    } else if table
        .page
        .ready()
        .is_some_and(|page| !page.result_set.rows.is_empty())
    {
        table.visual_anchor_row_index = Some(table.cursor.row_index);
    }
    Vec::new()
}

pub(crate) fn edit_selected_rows(state: &mut AppState) -> Vec<Command> {
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    let preset = match update_preset(workspace) {
        None => return Vec::new(),
        Some(Err(message)) => {
            state.notice = notice(NoticeLevel::Error, message);
            return Vec::new();
        }
        Some(Ok(preset)) => preset,
    };
    if let Some(table) = workspace.active_table_mut() {
        table.clear_selection();
    }
    workspace.push_query(Document {
        name: None,
        text: preset.text,
        revision: 1,
    });
    workspace.focus = Focus::Main;
    state.notice = notice(
        NoticeLevel::Info,
        format!(
            "UPDATE preset for {} row(s): fill :{} and press F5",
            preset.row_count, preset.parameter_name
        ),
    );
    Vec::new()
}

fn update_preset(workspace: &Workspace) -> Option<Result<UpdatePreset, String>> {
    let TabContent::Table(table) = &workspace.active_tab()?.content else {
        return None;
    };
    if table.tab != TableTab::Data {
        return None;
    }
    let result_set = &table.page.ready()?.result_set;
    let target_column = result_set.columns.get(table.cursor.column_index)?;
    let mut target_row_indexes = table.selected_row_indexes();
    if target_row_indexes.is_empty() {
        target_row_indexes.push(table.cursor.row_index);
    }
    let target_rows: Vec<(usize, &Vec<Cell>)> = target_row_indexes
        .into_iter()
        .filter_map(|row_index| result_set.rows.get(row_index).map(|row| (row_index, row)))
        .collect();
    if target_rows.is_empty() {
        return None;
    }
    let Some(key_columns) = key_columns(workspace, table, result_set) else {
        return Some(Err(
            "No primary key or id column to target the rows".to_owned()
        ));
    };
    let first_row_number = table.request.page_index * table.request.page_size + 1;
    let mut key_tuples = Vec::with_capacity(target_rows.len());
    for (row_index, row) in &target_rows {
        let Some(key_values) = key_values(row, &key_columns) else {
            return Some(Err(format!(
                "Row {} has a NULL key, cannot target it",
                first_row_number + row_index
            )));
        };
        key_tuples.push(match key_values.as_slice() {
            [single_value] => single_value.clone(),
            _ => format!("({})", key_values.join(", ")),
        });
    }
    let quoted_key_names: Vec<String> = key_columns
        .iter()
        .map(|(column_name, _)| quote_identifier(column_name))
        .collect();
    let key_target = match quoted_key_names.as_slice() {
        [single_name] => single_name.clone(),
        _ => format!("({})", quoted_key_names.join(", ")),
    };
    let parameter_name = match quote_identifier_if_needed(target_column) == *target_column {
        true => target_column.clone(),
        false => FALLBACK_PARAMETER_NAME.to_owned(),
    };
    let text = format!(
        "UPDATE {}.{}\nSET\n  {} = :{parameter_name}\nWHERE {key_target} IN ({})\n;",
        quote_identifier(&table.relation.schema),
        quote_identifier(&table.relation.name),
        quote_identifier(target_column),
        key_tuples.join(", "),
    );
    Some(Ok(UpdatePreset {
        text,
        row_count: target_rows.len(),
        parameter_name,
    }))
}

fn key_columns(
    workspace: &Workspace,
    table: &TableView,
    result_set: &ResultSet,
) -> Option<Vec<(String, usize)>> {
    let primary_key_names: Vec<&str> = workspace
        .catalog
        .ready()
        .and_then(|catalog| catalog.relation(&table.relation))
        .map(|relation| {
            relation
                .columns
                .iter()
                .filter(|column| column.is_primary_key)
                .map(|column| column.name.as_str())
                .collect()
        })
        .unwrap_or_default();
    let key_names = match primary_key_names.is_empty() {
        true => vec![FALLBACK_KEY_COLUMN],
        false => primary_key_names,
    };
    key_names
        .into_iter()
        .map(|key_name| {
            let column_index = result_set
                .columns
                .iter()
                .position(|column| column == key_name)?;
            Some((key_name.to_owned(), column_index))
        })
        .collect()
}

fn key_values(row: &[Cell], key_columns: &[(String, usize)]) -> Option<Vec<String>> {
    key_columns
        .iter()
        .map(|(_, column_index)| row.get(*column_index)?.as_deref().map(quote_literal))
        .collect()
}
