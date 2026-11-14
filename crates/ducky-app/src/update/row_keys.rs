use ducky_core::{Cell, ResultSet};
use ducky_sql::{quote_identifier, quote_literal};

use crate::state::{TableView, Workspace};

const FALLBACK_KEY_COLUMN: &str = "id";

pub(crate) struct RowKeys {
    pub(crate) key_target: String,
    pub(crate) key_tuples: Vec<String>,
}

pub(crate) fn target_row_indexes(table: &TableView, result_set: &ResultSet) -> Vec<usize> {
    let mut target_row_indexes = table.selected_row_indexes();
    if target_row_indexes.is_empty() {
        target_row_indexes.push(table.cursor.row_index);
    }
    target_row_indexes.retain(|row_index| *row_index < result_set.rows.len());
    target_row_indexes
}

pub(crate) fn row_keys(
    workspace: &Workspace,
    table: &TableView,
    result_set: &ResultSet,
    row_indexes: &[usize],
) -> Result<RowKeys, String> {
    let Some(key_columns) = key_columns(workspace, table, result_set) else {
        return Err("No primary key or id column to target the rows".to_owned());
    };
    let first_row_number = table.request.page_index * table.request.page_size + 1;
    let key_tuples = row_indexes
        .iter()
        .map(|row_index| {
            result_set
                .rows
                .get(*row_index)
                .and_then(|row| key_values(row, &key_columns))
                .map(|key_values| parenthesized(&key_values))
                .ok_or_else(|| {
                    format!(
                        "Row {} has a NULL key, cannot target it",
                        first_row_number + row_index
                    )
                })
        })
        .collect::<Result<Vec<String>, String>>()?;
    let quoted_key_names: Vec<String> = key_columns
        .iter()
        .map(|(column_name, _)| quote_identifier(column_name))
        .collect();
    Ok(RowKeys {
        key_target: parenthesized(&quoted_key_names),
        key_tuples,
    })
}

fn parenthesized(items: &[String]) -> String {
    match items {
        [single_item] => single_item.clone(),
        _ => format!("({})", items.join(", ")),
    }
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
