use super::*;

#[test]
fn splits_two_statements() {
    assert_eq!(
        split_statements("select 1; select 2"),
        vec!["select 1", "select 2"]
    );
}

#[test]
fn ignores_semicolons_in_strings_comments_and_dollar_quotes() {
    let statements = split_statements("select ';'; -- c;\n select $$a;b$$");
    assert_eq!(statements, vec!["select ';'", "-- c;\n select $$a;b$$"]);
}

#[test]
fn keeps_trailing_statement_without_semicolon() {
    assert_eq!(
        split_statements("select 1;\nselect 2\n"),
        vec!["select 1", "select 2"]
    );
}

#[test]
fn drops_empty_and_comment_only_pieces() {
    assert!(split_statements(";;  -- only comment").is_empty());
    assert!(split_statements("").is_empty());
}

#[test]
fn begin_atomic_body_is_not_cut() {
    let statements = split_statements(
        "create function f() returns int language sql begin atomic select 1; select 2; end; select 3",
    );
    assert_eq!(
        statements,
        vec![
            "create function f() returns int language sql begin atomic select 1; select 2; end",
            "select 3",
        ]
    );
}

#[test]
fn case_inside_atomic_block_opens_a_level() {
    let statements = split_statements(
        "create function f() returns int language sql begin atomic select case when true then 1 end; end; select 3",
    );
    assert_eq!(statements.len(), 2);
    assert_eq!(statements[1], "select 3");
}

#[test]
fn tokenizer_error_keeps_raw_text() {
    assert_eq!(split_statements("select 'abc"), vec!["select 'abc"]);
}

#[test]
fn tokenizer_error_after_semicolon_keeps_tail_statement() {
    assert_eq!(
        split_statements("select 1; select 'abc"),
        vec!["select 1", "select 'abc"]
    );
}
