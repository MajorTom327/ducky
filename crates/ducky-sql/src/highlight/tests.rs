use super::*;

fn position(line: usize, column: usize) -> TextPosition {
    TextPosition { line, column }
}

fn span(start: TextPosition, end: TextPosition, kind: TokenKind) -> HighlightSpan {
    HighlightSpan { start, end, kind }
}

#[test]
fn keyword_and_number() {
    assert_eq!(
        highlight("select 1"),
        vec![
            span(position(0, 0), position(0, 6), TokenKind::Keyword),
            span(position(0, 7), position(0, 8), TokenKind::Number),
        ]
    );
}

#[test]
fn single_line_comment() {
    assert_eq!(
        highlight("-- note"),
        vec![span(position(0, 0), position(0, 7), TokenKind::Comment)]
    );
}

#[test]
fn block_comment_over_two_lines_is_one_span() {
    assert_eq!(
        highlight("/* one\ntwo */"),
        vec![span(position(0, 0), position(1, 6), TokenKind::Comment)]
    );
}

#[test]
fn escaped_quote_string() {
    assert_eq!(
        highlight("'it''s'"),
        vec![span(position(0, 0), position(0, 7), TokenKind::String)]
    );
}

#[test]
fn quoted_identifier() {
    assert_eq!(
        highlight("\"Col\""),
        vec![span(
            position(0, 0),
            position(0, 5),
            TokenKind::QuotedIdentifier
        )]
    );
}

#[test]
fn identifiers_operators_and_punctuation() {
    let kinds: Vec<TokenKind> = highlight("a.b = $$x$$;")
        .iter()
        .map(|highlight_span| highlight_span.kind)
        .collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Identifier,
            TokenKind::Punctuation,
            TokenKind::Identifier,
            TokenKind::Operator,
            TokenKind::String,
            TokenKind::Punctuation,
        ]
    );
}

#[test]
fn unterminated_string_spans_to_the_end() {
    assert_eq!(
        highlight("select 'abc"),
        vec![
            span(position(0, 0), position(0, 6), TokenKind::Keyword),
            span(position(0, 7), position(0, 11), TokenKind::String),
        ]
    );
}

#[test]
fn unterminated_block_comment_spans_to_the_end() {
    let spans = highlight("select /* open\nstill");
    assert_eq!(
        spans.last(),
        Some(&span(position(0, 7), position(1, 5), TokenKind::Comment))
    );
}

#[test]
fn multi_byte_columns_are_counted_in_chars() {
    assert_eq!(
        highlight("'é' x"),
        vec![
            span(position(0, 0), position(0, 3), TokenKind::String),
            span(position(0, 4), position(0, 5), TokenKind::Identifier),
        ]
    );
}

#[test]
fn unterminated_prefixed_string_leaves_the_tail_unstyled() {
    assert_eq!(
        highlight("select E'abc"),
        vec![span(position(0, 0), position(0, 6), TokenKind::Keyword)]
    );
}
