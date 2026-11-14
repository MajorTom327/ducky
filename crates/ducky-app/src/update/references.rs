use ducky_core::{Cell, ForeignKeyLink, RelationRef};
use ducky_sql::{quote_identifier, quote_literal};

use crate::command::Command;
use crate::state::{
    AppState, Focus, Modal, Notice, NoticeLevel, TabContent, TableTab, TableView, Workspace,
};
use crate::update::tabs::reveal_schema;
use crate::update::{notice, workspace_mut};

pub(crate) fn go_to_referenced(state: &mut AppState) -> Vec<Command> {
    let page_size = state.page_size;
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    let Some(cursor_column) = cursor_column(workspace) else {
        return Vec::new();
    };
    let Some(link) = outgoing_link(workspace, &cursor_column) else {
        state.notice = notice(
            NoticeLevel::Info,
            format!("{cursor_column} is not a foreign key"),
        );
        return Vec::new();
    };
    let Some(values) = cursor_row_values(workspace, &link.columns) else {
        return Vec::new();
    };
    match filter_on(&link.referenced_columns, &values) {
        Ok(filter) => open_filtered(workspace, link.referenced, filter, page_size),
        Err(null_index) => {
            state.notice = null_notice(&link.columns[null_index]);
            Vec::new()
        }
    }
}

pub(crate) fn find_referencing(state: &mut AppState) -> Vec<Command> {
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    let Some(relation) = active_data_table(workspace).map(|table| table.relation.clone()) else {
        return Vec::new();
    };
    let Some(catalog) = workspace.catalog.ready() else {
        return Vec::new();
    };
    let mut links = catalog.referencing(&relation);
    match links.len() {
        0 => {
            state.notice = notice(
                NoticeLevel::Info,
                format!("No table references {}.{}", relation.schema, relation.name),
            );
            Vec::new()
        }
        1 => {
            let (referencing_relation, link) = links.remove(0);
            open_referencing(state, referencing_relation, link)
        }
        _ => {
            state.modal = Some(Modal::ReferencePicker {
                links,
                selected_link_index: 0,
            });
            Vec::new()
        }
    }
}

pub(crate) fn submit_picker(
    state: &mut AppState,
    mut links: Vec<(RelationRef, ForeignKeyLink)>,
    selected_link_index: usize,
) -> Vec<Command> {
    if selected_link_index >= links.len() {
        return Vec::new();
    }
    let (referencing_relation, link) = links.swap_remove(selected_link_index);
    open_referencing(state, referencing_relation, link)
}

fn open_referencing(
    state: &mut AppState,
    referencing_relation: RelationRef,
    link: ForeignKeyLink,
) -> Vec<Command> {
    let page_size = state.page_size;
    let Some(workspace) = workspace_mut(&mut state.screen) else {
        return Vec::new();
    };
    let Some(values) = cursor_row_values(workspace, &link.referenced_columns) else {
        return Vec::new();
    };
    match filter_on(&link.columns, &values) {
        Ok(filter) => open_filtered(workspace, referencing_relation, filter, page_size),
        Err(null_index) => {
            state.notice = null_notice(&link.referenced_columns[null_index]);
            Vec::new()
        }
    }
}

fn null_notice(column_name: &str) -> Option<Notice> {
    notice(
        NoticeLevel::Info,
        format!("{column_name} is NULL, nothing to follow"),
    )
}

fn active_data_table(workspace: &Workspace) -> Option<&TableView> {
    match &workspace.active_tab()?.content {
        TabContent::Table(table) if table.tab == TableTab::Data => Some(table),
        _ => None,
    }
}

fn cursor_column(workspace: &Workspace) -> Option<String> {
    let table = active_data_table(workspace)?;
    let result_set = &table.page.ready()?.result_set;
    result_set.columns.get(table.cursor.column_index).cloned()
}

fn outgoing_link(workspace: &Workspace, column_name: &str) -> Option<ForeignKeyLink> {
    let relation = &active_data_table(workspace)?.relation;
    workspace
        .catalog
        .ready()?
        .relation(relation)?
        .foreign_keys
        .iter()
        .filter(|link| link.columns.iter().any(|column| column == column_name))
        .min_by(|left, right| left.name.cmp(&right.name))
        .cloned()
}

fn cursor_row_values(workspace: &Workspace, column_names: &[String]) -> Option<Vec<Cell>> {
    let table = active_data_table(workspace)?;
    let result_set = &table.page.ready()?.result_set;
    let cursor_row = result_set.rows.get(table.cursor.row_index)?;
    column_names
        .iter()
        .map(|column_name| {
            let column_index = result_set
                .columns
                .iter()
                .position(|column| column == column_name)?;
            cursor_row.get(column_index).cloned()
        })
        .collect()
}

fn filter_on(column_names: &[String], values: &[Cell]) -> Result<String, usize> {
    let mut conditions = Vec::with_capacity(column_names.len());
    for (column_index, (column_name, value)) in column_names.iter().zip(values).enumerate() {
        let Some(value) = value else {
            return Err(column_index);
        };
        conditions.push(format!(
            "{} = {}",
            quote_identifier(column_name),
            quote_literal(value)
        ));
    }
    Ok(conditions.join(" AND "))
}

fn open_filtered(
    workspace: &mut Workspace,
    relation: RelationRef,
    filter: String,
    page_size: usize,
) -> Vec<Command> {
    reveal_schema(workspace, &relation.schema);
    workspace.focus = Focus::Main;
    let existing_tab_index = workspace.tabs.iter().position(|tab| {
        matches!(
            &tab.content,
            TabContent::Table(table)
                if table.relation == relation && table.request.filter.as_deref() == Some(filter.as_str())
        )
    });
    if let Some(tab_index) = existing_tab_index {
        workspace.active_tab_index = tab_index;
        return Vec::new();
    }
    let mut table = workspace.new_table_view(relation, page_size);
    table.request.filter = Some(filter);
    let request = table.request.clone();
    workspace.push_tab(TabContent::Table(table));
    vec![Command::LoadPage(request)]
}
