use ducky_core::{Column, Relation, RelationKind, Schema};

use super::*;

fn column(name: &str, data_type: &str) -> Column {
    Column {
        name: name.to_owned(),
        data_type: data_type.to_owned(),
        is_nullable: true,
        default_expression: None,
        is_primary_key: false,
    }
}

fn table(name: &str, columns: Vec<Column>) -> Relation {
    Relation {
        name: name.to_owned(),
        kind: RelationKind::Table,
        estimated_row_count: None,
        columns,
        foreign_keys: Vec::new(),
    }
}

fn fixture_catalog() -> Catalog {
    Catalog {
        schemas: vec![
            Schema {
                name: "public".to_owned(),
                relations: vec![
                    table(
                        "customers",
                        vec![
                            column("id", "integer"),
                            column("email", "text"),
                            column("name", "text"),
                        ],
                    ),
                    table(
                        "orders",
                        vec![
                            column("id", "integer"),
                            column("customer_id", "integer"),
                            column("status", "text"),
                        ],
                    ),
                ],
            },
            Schema {
                name: "analytics".to_owned(),
                relations: vec![table(
                    "events",
                    vec![column("id", "bigint"), column("name", "text")],
                )],
            },
        ],
    }
}

fn complete_at_cursor(sql_with_cursor: &str) -> Completion {
    let cursor = sql_with_cursor.find('|').unwrap();
    let sql = sql_with_cursor.replace('|', "");
    complete(&sql, cursor, &fixture_catalog(), "public")
}

fn labels(completion: &Completion) -> Vec<&str> {
    completion
        .items
        .iter()
        .map(|item| item.label.as_str())
        .collect()
}

#[test]
fn relation_after_from() {
    let completion = complete_at_cursor("select * from cu|");
    assert_eq!(completion.replace_start, 14);
    let first_item = &completion.items[0];
    assert_eq!(first_item.label, "customers");
    assert_eq!(first_item.kind, CompletionKind::Relation);
    assert_eq!(first_item.detail.as_deref(), Some("table"));
}

#[test]
fn schema_names_follow_relations_after_from() {
    let completion = complete_at_cursor("select * from |");
    assert_eq!(
        labels(&completion),
        vec!["customers", "orders", "analytics", "public"]
    );
}

#[test]
fn alias_qualified_column() {
    let completion = complete_at_cursor("select c.em| from customers c");
    assert_eq!(completion.replace_start, 9);
    assert_eq!(
        completion.items,
        vec![CompletionItem {
            label: "email".to_owned(),
            kind: CompletionKind::Column,
            detail: Some("text".to_owned()),
        }]
    );
}

#[test]
fn relation_name_qualified_column() {
    let completion = complete_at_cursor("select orders.st| from orders");
    assert_eq!(labels(&completion), vec!["status"]);
}

#[test]
fn schema_qualified_relation() {
    let completion = complete_at_cursor("select analytics.|");
    assert_eq!(labels(&completion), vec!["events"]);
    assert_eq!(completion.items[0].kind, CompletionKind::Relation);
}

#[test]
fn unknown_qualifier_has_no_items() {
    assert!(complete_at_cursor("select nope.|").items.is_empty());
}

#[test]
fn keyword_prefix() {
    let completion = complete_at_cursor("sel|");
    assert_eq!(completion.replace_start, 0);
    assert!(
        completion
            .items
            .iter()
            .any(|item| item.label == "SELECT" && item.kind == CompletionKind::Keyword)
    );
}

#[test]
fn columns_come_before_keywords() {
    let completion = complete_at_cursor("select na| from customers");
    assert_eq!(completion.items[0].label, "name");
    assert_eq!(completion.items[0].kind, CompletionKind::Column);
    assert!(
        completion
            .items
            .iter()
            .skip(1)
            .all(|item| item.kind == CompletionKind::Keyword)
    );
}

#[test]
fn only_current_statement_relations_are_used() {
    let completion = complete_at_cursor("select 1; select st| from orders");
    assert_eq!(completion.items[0].label, "status");
    assert!(
        completion
            .items
            .iter()
            .filter(|item| item.kind == CompletionKind::Column)
            .all(|item| item.label == "status")
    );
    let other_statement = complete_at_cursor("select em| from orders; select 1 from customers");
    assert!(
        other_statement
            .items
            .iter()
            .all(|item| item.label != "email")
    );
}

#[test]
fn empty_prefix_offers_only_columns() {
    let completion = complete_at_cursor("select | from orders");
    assert_eq!(labels(&completion), vec!["customer_id", "id", "status"]);
}

#[test]
fn columns_are_deduplicated_across_relations() {
    let completion = complete_at_cursor("select i| from customers c join orders o on true");
    let id_items = completion
        .items
        .iter()
        .filter(|item| item.label == "id")
        .count();
    assert_eq!(id_items, 1);
}

#[test]
fn from_list_after_comma_and_schema_qualified_ref() {
    let completion = complete_at_cursor("select e.na| from customers c, analytics.events e");
    assert_eq!(labels(&completion), vec!["name"]);
    assert_eq!(completion.items[0].detail.as_deref(), Some("text"));
}

#[test]
fn items_are_capped() {
    let completion = complete_at_cursor("select a|");
    assert!(completion.items.len() <= 50);
    assert!(!completion.items.is_empty());
}

#[test]
fn cursor_in_middle_uses_text_before_cursor() {
    let completion = complete_at_cursor("select * from cu|stomers");
    assert_eq!(completion.replace_start, 14);
    assert_eq!(completion.items[0].label, "customers");
}
