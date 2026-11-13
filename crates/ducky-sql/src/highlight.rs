use sqlparser::keywords::Keyword;
use sqlparser::tokenizer::{Token, Whitespace};

use crate::position::{TextPosition, position_at};
use crate::tokens::tokenize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Keyword,
    Identifier,
    QuotedIdentifier,
    String,
    Number,
    Comment,
    Operator,
    Punctuation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighlightSpan {
    pub start: TextPosition,
    pub end: TextPosition,
    pub kind: TokenKind,
}

pub fn highlight(sql: &str) -> Vec<HighlightSpan> {
    let tokenized_sql = tokenize(sql);
    let mut spans: Vec<HighlightSpan> = tokenized_sql
        .tokens
        .iter()
        .filter(|sql_token| sql_token.start < sql_token.end)
        .filter_map(|sql_token| {
            token_kind(&sql_token.token).map(|kind| HighlightSpan {
                start: sql_token.start_position,
                end: sql_token.end_position,
                kind,
            })
        })
        .collect();
    if tokenized_sql.had_error
        && let Some(tail_span) = untokenized_tail_span(sql, tokenized_sql.tail_start())
    {
        spans.push(tail_span);
    }
    spans
}

fn untokenized_tail_span(sql: &str, tail_start: usize) -> Option<HighlightSpan> {
    let tail = &sql[tail_start..];
    let tail_content = tail.trim_start();
    let kind = if tail_content.starts_with("/*") {
        TokenKind::Comment
    } else if tail_content.starts_with(['\'', '"', '$']) {
        TokenKind::String
    } else {
        return None;
    };
    let content_start = tail_start + (tail.len() - tail_content.len());
    Some(HighlightSpan {
        start: position_at(sql, content_start),
        end: position_at(sql, sql.len()),
        kind,
    })
}

fn token_kind(token: &Token) -> Option<TokenKind> {
    let kind = match token {
        Token::Word(word) if word.quote_style.is_some() => TokenKind::QuotedIdentifier,
        Token::Word(word) if word.keyword != Keyword::NoKeyword => TokenKind::Keyword,
        Token::Word(_) => TokenKind::Identifier,
        Token::Number(..) => TokenKind::Number,
        Token::SingleQuotedString(_)
        | Token::DoubleQuotedString(_)
        | Token::TripleSingleQuotedString(_)
        | Token::TripleDoubleQuotedString(_)
        | Token::DollarQuotedString(_)
        | Token::SingleQuotedByteStringLiteral(_)
        | Token::DoubleQuotedByteStringLiteral(_)
        | Token::TripleSingleQuotedByteStringLiteral(_)
        | Token::TripleDoubleQuotedByteStringLiteral(_)
        | Token::SingleQuotedRawStringLiteral(_)
        | Token::DoubleQuotedRawStringLiteral(_)
        | Token::TripleSingleQuotedRawStringLiteral(_)
        | Token::TripleDoubleQuotedRawStringLiteral(_)
        | Token::NationalStringLiteral(_)
        | Token::QuoteDelimitedStringLiteral(_)
        | Token::NationalQuoteDelimitedStringLiteral(_)
        | Token::EscapedStringLiteral(_)
        | Token::UnicodeStringLiteral(_)
        | Token::HexStringLiteral(_) => TokenKind::String,
        Token::Whitespace(
            Whitespace::SingleLineComment { .. } | Whitespace::MultiLineComment(_),
        ) => TokenKind::Comment,
        Token::Whitespace(_) => return None,
        Token::Comma
        | Token::SemiColon
        | Token::LParen
        | Token::RParen
        | Token::LBracket
        | Token::RBracket
        | Token::LBrace
        | Token::RBrace
        | Token::Period
        | Token::Colon => TokenKind::Punctuation,
        _ => TokenKind::Operator,
    };
    Some(kind)
}

#[cfg(test)]
mod tests;
