use sqlparser::keywords::Keyword;
use sqlparser::tokenizer::Token;

use crate::tokens::{SqlToken, tokenize};

pub fn split_statements(sql: &str) -> Vec<String> {
    let tokenized_sql = tokenize(sql);
    let tokens = &tokenized_sql.tokens;
    let mut statements = Vec::new();
    let mut piece_start = 0;
    let mut piece_first_token_index = 0;
    for semicolon_index in top_level_semicolon_indices(tokens) {
        if has_statement_content(&tokens[piece_first_token_index..semicolon_index]) {
            statements.push(
                sql[piece_start..tokens[semicolon_index].start]
                    .trim()
                    .to_owned(),
            );
        }
        piece_start = tokens[semicolon_index].end;
        piece_first_token_index = semicolon_index + 1;
    }
    let has_untokenized_tail =
        tokenized_sql.had_error && !sql[tokenized_sql.tail_start()..].trim().is_empty();
    if has_untokenized_tail || has_statement_content(&tokens[piece_first_token_index..]) {
        statements.push(sql[piece_start..].trim().to_owned());
    }
    statements
}

pub(crate) fn top_level_semicolon_indices(tokens: &[SqlToken]) -> Vec<usize> {
    let mut semicolon_indices = Vec::new();
    let mut atomic_block_depth = 0usize;
    for (token_index, sql_token) in tokens.iter().enumerate() {
        match &sql_token.token {
            Token::Word(word) if atomic_block_depth == 0 && word.keyword == Keyword::ATOMIC => {
                atomic_block_depth = 1;
            }
            Token::Word(word) if atomic_block_depth > 0 && word.keyword == Keyword::CASE => {
                atomic_block_depth += 1;
            }
            Token::Word(word) if atomic_block_depth > 0 && word.keyword == Keyword::END => {
                atomic_block_depth -= 1;
            }
            Token::SemiColon if atomic_block_depth == 0 => semicolon_indices.push(token_index),
            _ => {}
        }
    }
    semicolon_indices
}

fn has_statement_content(tokens: &[SqlToken]) -> bool {
    tokens
        .iter()
        .any(|sql_token| !matches!(sql_token.token, Token::Whitespace(_)))
}

#[cfg(test)]
mod tests;
