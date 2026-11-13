use sqlparser::tokenizer::Token;

use crate::identifier::quote_literal;
use crate::tokens::{SqlToken, tokenize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterValue {
    Text(String),
    Raw(String),
}

struct ParameterOccurrence {
    name: String,
    start: usize,
    end: usize,
}

pub fn find_parameters(sql: &str) -> Vec<String> {
    let mut parameter_names: Vec<String> = Vec::new();
    for occurrence in parameter_occurrences(sql) {
        if !parameter_names.contains(&occurrence.name) {
            parameter_names.push(occurrence.name);
        }
    }
    parameter_names
}

pub fn bind_parameters(sql: &str, values: &[(String, ParameterValue)]) -> String {
    let mut bound_sql = String::with_capacity(sql.len());
    let mut copied_until = 0;
    for occurrence in parameter_occurrences(sql) {
        let Some((_, value)) = values.iter().find(|(name, _)| *name == occurrence.name) else {
            continue;
        };
        bound_sql.push_str(&sql[copied_until..occurrence.start]);
        match value {
            ParameterValue::Text(text) => bound_sql.push_str(&quote_literal(text)),
            ParameterValue::Raw(raw_sql) => bound_sql.push_str(raw_sql),
        }
        copied_until = occurrence.end;
    }
    bound_sql.push_str(&sql[copied_until..]);
    bound_sql
}

fn parameter_occurrences(sql: &str) -> Vec<ParameterOccurrence> {
    let tokens = tokenize(sql).tokens;
    let mut occurrences = Vec::new();
    let mut bracket_depth = 0usize;
    let mut previous_token: Option<&Token> = None;
    for (token_index, sql_token) in tokens.iter().enumerate() {
        match &sql_token.token {
            Token::LBracket => bracket_depth += 1,
            Token::RBracket => bracket_depth = bracket_depth.saturating_sub(1),
            Token::Placeholder(placeholder) if is_positional(placeholder) => {
                occurrences.push(ParameterOccurrence {
                    name: placeholder.clone(),
                    start: sql_token.start,
                    end: sql_token.end,
                });
            }
            Token::Colon if bracket_depth == 0 || !previous_token.is_some_and(is_slice_bound) => {
                if let Some(occurrence) = named_occurrence(sql_token, tokens.get(token_index + 1)) {
                    occurrences.push(occurrence);
                }
            }
            _ => {}
        }
        if !matches!(sql_token.token, Token::Whitespace(_)) {
            previous_token = Some(&sql_token.token);
        }
    }
    occurrences
}

fn is_slice_bound(token: &Token) -> bool {
    matches!(
        token,
        Token::Number(..)
            | Token::Word(_)
            | Token::Placeholder(_)
            | Token::RParen
            | Token::RBracket
    )
}

fn named_occurrence(
    colon: &SqlToken,
    next_token: Option<&SqlToken>,
) -> Option<ParameterOccurrence> {
    let name_token = next_token?;
    let Token::Word(word) = &name_token.token else {
        return None;
    };
    if word.quote_style.is_some() || name_token.start != colon.end || !is_identifier(&word.value) {
        return None;
    }
    Some(ParameterOccurrence {
        name: format!(":{}", word.value),
        start: colon.start,
        end: name_token.end,
    })
}

fn is_positional(placeholder: &str) -> bool {
    placeholder.strip_prefix('$').is_some_and(|digits| {
        !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
    })
}

fn is_identifier(name: &str) -> bool {
    let mut name_chars = name.chars();
    name_chars
        .next()
        .is_some_and(|first_char| first_char.is_ascii_alphabetic() || first_char == '_')
        && name_chars.all(|name_char| name_char.is_ascii_alphanumeric() || name_char == '_')
}

#[cfg(test)]
mod tests;
