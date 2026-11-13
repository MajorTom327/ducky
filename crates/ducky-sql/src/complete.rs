mod items;

use ducky_core::Catalog;
use sqlparser::keywords::Keyword;
use sqlparser::tokenizer::Token;

use crate::position::floor_char_boundary;
use crate::split::top_level_semicolon_indices;
use crate::table_refs::{TableRef, table_refs};
use crate::tokens::{SqlToken, tokenize};
use items::{column_items, keyword_items, relation_items, schema_items};

const MAX_COMPLETION_ITEMS: usize = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Column,
    Relation,
    Schema,
    Keyword,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    pub replace_start: usize,
    pub items: Vec<CompletionItem>,
}

pub fn complete(sql: &str, cursor: usize, catalog: &Catalog, default_schema: &str) -> Completion {
    let cursor = floor_char_boundary(sql, cursor);
    let prefix_length = trailing_identifier_length(&sql[..cursor]);
    let replace_start = cursor - prefix_length;
    let prefix = &sql[replace_start..cursor];
    let tokenized_sql = tokenize(sql);
    let statement_tokens = statement_tokens_around(&tokenized_sql.tokens, replace_start, cursor);
    let statement_refs = table_refs(statement_tokens, default_schema);
    let item_groups = if sql[..replace_start].ends_with('.') {
        vec![qualified_items(
            sql,
            replace_start - 1,
            prefix,
            catalog,
            &statement_refs,
        )]
    } else if is_relation_position(statement_tokens, replace_start) {
        vec![
            relation_items(catalog.schema(default_schema), prefix),
            schema_items(catalog, prefix),
        ]
    } else if prefix.is_empty() {
        vec![column_items(catalog, &statement_refs, prefix)]
    } else {
        vec![
            column_items(catalog, &statement_refs, prefix),
            relation_items(catalog.schema(default_schema), prefix),
            keyword_items(prefix),
        ]
    };
    let items = item_groups
        .into_iter()
        .flat_map(|mut item_group| {
            item_group.sort_by_cached_key(|item| (item.label.to_lowercase(), item.label.clone()));
            item_group
        })
        .take(MAX_COMPLETION_ITEMS)
        .collect();
    Completion {
        replace_start,
        items,
    }
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn trailing_identifier_length(text: &str) -> usize {
    text.bytes()
        .rev()
        .take_while(|byte| is_identifier_byte(*byte))
        .count()
}

fn matches_prefix(name: &str, prefix: &str) -> bool {
    name.len() >= prefix.len()
        && name.as_bytes()[..prefix.len()].eq_ignore_ascii_case(prefix.as_bytes())
}

fn statement_tokens_around(
    tokens: &[SqlToken],
    replace_start: usize,
    cursor: usize,
) -> &[SqlToken] {
    let semicolon_indices = top_level_semicolon_indices(tokens);
    let first_token_index = semicolon_indices
        .iter()
        .rev()
        .find(|semicolon_index| tokens[**semicolon_index].end <= replace_start)
        .map_or(0, |semicolon_index| semicolon_index + 1);
    let end_token_index = semicolon_indices
        .iter()
        .find(|semicolon_index| tokens[**semicolon_index].start >= cursor)
        .copied()
        .unwrap_or(tokens.len());
    tokens
        .get(first_token_index..end_token_index)
        .unwrap_or_default()
}

fn is_relation_position(statement_tokens: &[SqlToken], replace_start: usize) -> bool {
    let nearest_keyword = statement_tokens
        .iter()
        .rev()
        .filter(|sql_token| sql_token.end <= replace_start)
        .find_map(|sql_token| match &sql_token.token {
            Token::Word(word)
                if word.quote_style.is_none() && word.keyword != Keyword::NoKeyword =>
            {
                Some(word.keyword)
            }
            _ => None,
        });
    matches!(
        nearest_keyword,
        Some(
            Keyword::FROM
                | Keyword::JOIN
                | Keyword::INTO
                | Keyword::UPDATE
                | Keyword::TABLE
                | Keyword::TRUNCATE
        )
    )
}

fn qualifier_before(sql: &str, dot_offset: usize) -> Option<String> {
    let text_before_dot = &sql[..dot_offset];
    if let Some(quoted_text) = text_before_dot.strip_suffix('"') {
        let opening_quote_offset = quoted_text.rfind('"')?;
        return Some(quoted_text[opening_quote_offset + 1..].to_owned());
    }
    let qualifier_length = trailing_identifier_length(text_before_dot);
    (qualifier_length > 0).then(|| text_before_dot[dot_offset - qualifier_length..].to_lowercase())
}

fn qualified_items(
    sql: &str,
    dot_offset: usize,
    prefix: &str,
    catalog: &Catalog,
    statement_refs: &[TableRef],
) -> Vec<CompletionItem> {
    let Some(qualifier) = qualifier_before(sql, dot_offset) else {
        return Vec::new();
    };
    if let Some(qualified_schema) = catalog
        .schemas
        .iter()
        .find(|schema| schema.name.eq_ignore_ascii_case(&qualifier))
    {
        return relation_items(Some(qualified_schema), prefix);
    }
    let qualified_ref = statement_refs.iter().find(|table_ref| {
        table_ref
            .alias
            .as_deref()
            .is_some_and(|alias| alias.eq_ignore_ascii_case(&qualifier))
    });
    let qualified_ref = qualified_ref.or_else(|| {
        statement_refs
            .iter()
            .find(|table_ref| table_ref.relation.name.eq_ignore_ascii_case(&qualifier))
    });
    match qualified_ref {
        Some(table_ref) => column_items(catalog, std::slice::from_ref(table_ref), prefix),
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests;
