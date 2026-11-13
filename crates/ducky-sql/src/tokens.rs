use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::tokenizer::{Location, Token, TokenWithSpan, Tokenizer};

use crate::position::{TextPosition, position_at};

pub(crate) struct SqlToken {
    pub(crate) token: Token,
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) start_position: TextPosition,
    pub(crate) end_position: TextPosition,
}

pub(crate) struct TokenizedSql {
    pub(crate) tokens: Vec<SqlToken>,
    pub(crate) had_error: bool,
}

impl TokenizedSql {
    pub(crate) fn tail_start(&self) -> usize {
        self.tokens.last().map_or(0, |last_token| last_token.end)
    }
}

pub(crate) fn tokenize(sql: &str) -> TokenizedSql {
    let dialect = PostgreSqlDialect {};
    let mut raw_tokens: Vec<TokenWithSpan> = Vec::new();
    let had_error = Tokenizer::new(&dialect, sql)
        .tokenize_with_location_into_buf(&mut raw_tokens)
        .is_err();
    raw_tokens.sort_by_key(|raw_token| raw_token.span.start);
    let mut locator = Locator::new(sql);
    let starts: Vec<(usize, TextPosition)> = raw_tokens
        .iter()
        .map(|raw_token| locator.locate(raw_token.span.start))
        .collect();
    let final_end = match raw_tokens.last() {
        Some(last_token) if had_error => locator.locate(last_token.span.end),
        _ => (sql.len(), position_at(sql, sql.len())),
    };
    let tokens = raw_tokens
        .into_iter()
        .enumerate()
        .map(|(token_index, raw_token)| {
            let (start, start_position) = starts[token_index];
            let (end, end_position) = starts.get(token_index + 1).copied().unwrap_or(final_end);
            let (end, end_position) = if end < start {
                (start, start_position)
            } else {
                (end, end_position)
            };
            SqlToken {
                token: raw_token.token,
                start,
                end,
                start_position,
                end_position,
            }
        })
        .collect();
    TokenizedSql { tokens, had_error }
}

struct Locator<'a> {
    sql: &'a str,
    line: u64,
    column: u64,
    offset: usize,
}

impl<'a> Locator<'a> {
    fn new(sql: &'a str) -> Locator<'a> {
        Locator {
            sql,
            line: 1,
            column: 1,
            offset: 0,
        }
    }

    fn locate(&mut self, location: Location) -> (usize, TextPosition) {
        let target = (location.line, location.column);
        if target < (self.line, self.column) {
            self.line = 1;
            self.column = 1;
            self.offset = 0;
        }
        while (self.line, self.column) < target {
            let Some(next_char) = self.sql[self.offset..].chars().next() else {
                break;
            };
            self.offset += next_char.len_utf8();
            if next_char == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
        let position = TextPosition {
            line: (self.line - 1) as usize,
            column: (self.column - 1) as usize,
        };
        (self.offset, position)
    }
}
