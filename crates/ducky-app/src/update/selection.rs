use ducky_sql::{quote_identifier, quote_identifier_if_needed};

use crate::command::Command;
use crate::msg::GridMove;
use crate::state::{
    AppState, Document, Focus, NoticeLevel, TabContent, TableTab, TableView, Workspace,
};
use crate::update::grid::move_cursor;
use crate::update::row_keys::{row_keys, target_row_indexes};
use crate::update::{notice, workspace_mut};

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
    let target_row_indexes = target_row_indexes(table, result_set);
    if target_row_indexes.is_empty() {
        return None;
    }
    let row_keys = match row_keys(workspace, table, result_set, &target_row_indexes) {
        Ok(row_keys) => row_keys,
        Err(message) => return Some(Err(message)),
    };
    let parameter_name = match quote_identifier_if_needed(target_column) == *target_column {
        true => target_column.clone(),
        false => FALLBACK_PARAMETER_NAME.to_owned(),
    };
    let text = format!(
        "UPDATE {}.{}\nSET\n  {} = :{parameter_name}\nWHERE {} IN ({})\n;",
        quote_identifier(&table.relation.schema),
        quote_identifier(&table.relation.name),
        quote_identifier(target_column),
        row_keys.key_target,
        row_keys.key_tuples.join(", "),
    );
    Some(Ok(UpdatePreset {
        text,
        row_count: target_row_indexes.len(),
        parameter_name,
    }))
}
