use ducky_core::{Catalog, RelationKind, Schema};
use sqlparser::keywords::ALL_KEYWORDS;

use super::{CompletionItem, CompletionKind, matches_prefix};
use crate::identifier::quote_identifier_if_needed;
use crate::table_refs::TableRef;

pub(super) fn column_items(
    catalog: &Catalog,
    statement_refs: &[TableRef],
    prefix: &str,
) -> Vec<CompletionItem> {
    let mut seen_column_names: Vec<&str> = Vec::new();
    let mut items = Vec::new();
    for table_ref in statement_refs {
        let Some(relation) = catalog.relation(&table_ref.relation) else {
            continue;
        };
        for column in &relation.columns {
            if seen_column_names.contains(&column.name.as_str())
                || !matches_prefix(&column.name, prefix)
            {
                continue;
            }
            seen_column_names.push(&column.name);
            items.push(CompletionItem {
                label: quote_identifier_if_needed(&column.name),
                kind: CompletionKind::Column,
                detail: Some(column.data_type.clone()),
            });
        }
    }
    items
}

pub(super) fn relation_items(schema: Option<&Schema>, prefix: &str) -> Vec<CompletionItem> {
    schema
        .map(|schema| schema.relations.as_slice())
        .unwrap_or_default()
        .iter()
        .filter(|relation| matches_prefix(&relation.name, prefix))
        .map(|relation| CompletionItem {
            label: quote_identifier_if_needed(&relation.name),
            kind: CompletionKind::Relation,
            detail: Some(relation_kind_label(relation.kind).to_owned()),
        })
        .collect()
}

fn relation_kind_label(kind: RelationKind) -> &'static str {
    match kind {
        RelationKind::Table => "table",
        RelationKind::View => "view",
        RelationKind::MaterializedView => "materialized view",
        RelationKind::ForeignTable => "foreign table",
    }
}

pub(super) fn schema_items(catalog: &Catalog, prefix: &str) -> Vec<CompletionItem> {
    catalog
        .schemas
        .iter()
        .filter(|schema| matches_prefix(&schema.name, prefix))
        .map(|schema| CompletionItem {
            label: quote_identifier_if_needed(&schema.name),
            kind: CompletionKind::Schema,
            detail: None,
        })
        .collect()
}

pub(super) fn keyword_items(prefix: &str) -> Vec<CompletionItem> {
    ALL_KEYWORDS
        .iter()
        .filter(|keyword| matches_prefix(keyword, prefix))
        .map(|keyword| CompletionItem {
            label: keyword.to_uppercase(),
            kind: CompletionKind::Keyword,
            detail: None,
        })
        .collect()
}
