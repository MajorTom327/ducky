use ducky_core::{Relation, RelationKind, RelationRef, ToolStatus};
use tui_input::Input;

use crate::state::tabs::TabId;
use crate::state::workspace::Workspace;

const MAX_UNFILTERED_ITEMS: usize = 200;
const MAX_FILTERED_ITEMS: usize = 50;

#[derive(Debug, Clone, Default)]
pub struct CommandPalette {
    pub input: Input,
    pub selected_item_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteItem {
    pub label: String,
    pub detail: Option<String>,
    pub action: PaletteAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteAction {
    SwitchTab(TabId),
    NewQuery,
    SqlFiles,
    Dump,
    Restore,
    ChooseSchema,
    RefreshCatalog,
    Reconnect,
    Disconnect,
    Help,
    OpenRelation(RelationRef),
}

pub(crate) fn palette_items(workspace: &Workspace, query: &str) -> Vec<PaletteItem> {
    let candidates = candidates(workspace);
    if query.is_empty() {
        return candidates.into_iter().take(MAX_UNFILTERED_ITEMS).collect();
    }
    let lowercase_query = query.to_lowercase();
    let mut ranked_items: Vec<(u8, PaletteItem)> = candidates
        .into_iter()
        .filter_map(|item| match_rank(&item.label, &lowercase_query).map(|rank| (rank, item)))
        .collect();
    ranked_items.sort_by_key(|(rank, _)| *rank);
    ranked_items
        .into_iter()
        .take(MAX_FILTERED_ITEMS)
        .map(|(_, item)| item)
        .collect()
}

fn match_rank(label: &str, lowercase_query: &str) -> Option<u8> {
    let lowercase_label = label.to_lowercase();
    let mut label_chars = lowercase_label.chars();
    let is_subsequence = lowercase_query
        .chars()
        .all(|query_char| label_chars.any(|label_char| label_char == query_char));
    if !is_subsequence {
        return None;
    }
    let label_tail = lowercase_label
        .rsplit_once('.')
        .map_or(lowercase_label.as_str(), |(_, tail)| tail);
    if lowercase_label.starts_with(lowercase_query) || label_tail.starts_with(lowercase_query) {
        Some(0)
    } else if lowercase_label.contains(lowercase_query) {
        Some(1)
    } else {
        Some(2)
    }
}

fn candidates(workspace: &Workspace) -> Vec<PaletteItem> {
    let mut items: Vec<PaletteItem> = workspace
        .tabs
        .iter()
        .map(|tab| PaletteItem {
            label: tab.title(),
            detail: Some("open tab".to_owned()),
            action: PaletteAction::SwitchTab(tab.id),
        })
        .collect();
    items.extend(action_items(workspace));
    let Some(catalog) = workspace.catalog.ready() else {
        return items;
    };
    let current_schema = catalog.schema(&workspace.schema_name);
    let other_schemas = catalog
        .schemas
        .iter()
        .filter(|schema| schema.name != workspace.schema_name);
    for schema in current_schema.into_iter().chain(other_schemas) {
        let is_current_schema = schema.name == workspace.schema_name;
        let mut relations: Vec<&Relation> = schema.relations.iter().collect();
        relations.sort_by(|left, right| left.name.cmp(&right.name));
        items.extend(relations.into_iter().map(|relation| PaletteItem {
            label: if is_current_schema {
                relation.name.clone()
            } else {
                format!("{}.{}", schema.name, relation.name)
            },
            detail: Some(relation_detail(relation)),
            action: PaletteAction::OpenRelation(RelationRef {
                schema: schema.name.clone(),
                name: relation.name.clone(),
            }),
        }));
    }
    items
}

fn action_items(workspace: &Workspace) -> Vec<PaletteItem> {
    let tool_detail = |tool: &ToolStatus| match tool {
        ToolStatus::Available(_) => None,
        ToolStatus::Missing(_) => Some("unavailable".to_owned()),
    };
    [
        ("New SQL query", None, PaletteAction::NewQuery),
        ("SQL files", None, PaletteAction::SqlFiles),
        (
            "Dump database",
            tool_detail(&workspace.tools.pg_dump),
            PaletteAction::Dump,
        ),
        (
            "Restore dump",
            tool_detail(&workspace.tools.pg_restore),
            PaletteAction::Restore,
        ),
        ("Switch schema", None, PaletteAction::ChooseSchema),
        ("Refresh catalog", None, PaletteAction::RefreshCatalog),
        ("Reconnect", None, PaletteAction::Reconnect),
        ("Disconnect", None, PaletteAction::Disconnect),
        ("Help", None, PaletteAction::Help),
    ]
    .into_iter()
    .map(|(label, detail, action)| PaletteItem {
        label: label.to_owned(),
        detail,
        action,
    })
    .collect()
}

fn relation_detail(relation: &Relation) -> String {
    match relation.kind {
        RelationKind::Table => match relation.estimated_row_count {
            Some(row_count) => format!("table · ~{row_count} rows"),
            None => "table".to_owned(),
        },
        RelationKind::View => "view".to_owned(),
        RelationKind::MaterializedView => "materialized view".to_owned(),
        RelationKind::ForeignTable => "foreign table".to_owned(),
    }
}
