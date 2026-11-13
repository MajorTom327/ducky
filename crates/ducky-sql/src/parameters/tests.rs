use super::*;

fn text(name: &str, value: &str) -> (String, ParameterValue) {
    (name.to_owned(), ParameterValue::Text(value.to_owned()))
}

fn raw(name: &str, value: &str) -> (String, ParameterValue) {
    (name.to_owned(), ParameterValue::Raw(value.to_owned()))
}

#[test]
fn finds_unique_named_parameters_in_first_appearance_order() {
    assert_eq!(
        find_parameters("select * from t where id = :id and name = :name or id = :id"),
        vec![":id", ":name"]
    );
}

#[test]
fn finds_positional_parameters() {
    assert_eq!(
        find_parameters("select * from t where a = $2 and b = $1 or c = $2"),
        vec!["$2", "$1"]
    );
}

#[test]
fn mixes_named_and_positional_parameters() {
    assert_eq!(find_parameters("select :a, $1, :b"), vec![":a", "$1", ":b"]);
}

#[test]
fn ignores_casts() {
    assert!(find_parameters("select id::int, '1'::text from t").is_empty());
}

#[test]
fn ignores_strings_and_quoted_identifiers() {
    assert!(find_parameters("select ':id', \"col:x\" from t").is_empty());
}

#[test]
fn ignores_comments() {
    assert!(find_parameters("select 1 -- where id = :id\n/* :other */").is_empty());
}

#[test]
fn ignores_dollar_quoted_bodies() {
    assert!(find_parameters("select $$ where id = :x $$, $tag$ :y $1 $tag$").is_empty());
}

#[test]
fn ignores_array_slices() {
    assert!(
        find_parameters("select arr[1:n], arr[lo:hi], arr[ 1 : n ], f(x)[1:2], m[1][2:3] from t")
            .is_empty()
    );
}

#[test]
fn finds_parameters_inside_array_constructors() {
    assert_eq!(
        find_parameters("select ARRAY[:a, :b] where id = ANY(ARRAY[:ids])"),
        vec![":a", ":b", ":ids"]
    );
}

#[test]
fn binds_parameters_inside_array_constructors_and_keeps_slices() {
    assert_eq!(
        bind_parameters(
            "select arr[1:n], ARRAY[:a, :b]",
            &[raw(":a", "1"), text(":b", "x"), text(":n", "9")]
        ),
        "select arr[1:n], ARRAY[1, 'x']"
    );
}

#[test]
fn ignores_colon_separated_from_name() {
    assert!(find_parameters("select : name").is_empty());
}

#[test]
fn finds_parameter_after_slice() {
    assert_eq!(
        find_parameters("select arr[1:2] from t where id = :id"),
        vec![":id"]
    );
}

#[test]
fn binds_text_as_quoted_literal() {
    assert_eq!(
        bind_parameters(
            "select * from t where name = :name",
            &[text(":name", "it's")]
        ),
        "select * from t where name = 'it''s'"
    );
}

#[test]
fn binds_raw_verbatim() {
    assert_eq!(
        bind_parameters(
            "select * from t where at < :at and id in :ids",
            &[raw(":at", "now()"), raw(":ids", "(1, 2, 3)")]
        ),
        "select * from t where at < now() and id in (1, 2, 3)"
    );
}

#[test]
fn binds_positional_parameters() {
    assert_eq!(
        bind_parameters("select $1, $2", &[text("$1", "42"), raw("$2", "NULL")]),
        "select '42', NULL"
    );
}

#[test]
fn leaves_parameters_without_value_untouched() {
    assert_eq!(
        bind_parameters("select :a, :b, $1", &[text(":a", "x")]),
        "select 'x', :b, $1"
    );
}

#[test]
fn binds_every_occurrence_across_a_multi_statement_script() {
    assert_eq!(
        bind_parameters(
            "insert into t(id) values (:id);\nselect * from t where id = :id and s = ':id' and c = c::text;",
            &[text(":id", "7")]
        ),
        "insert into t(id) values ('7');\nselect * from t where id = '7' and s = ':id' and c = c::text;"
    );
}

#[test]
fn bind_without_parameters_returns_input() {
    let sql = "select 'a:b', arr[1:n] from t";
    assert_eq!(bind_parameters(sql, &[text(":n", "1")]), sql);
}

#[test]
fn ignores_quoted_and_non_ascii_names_after_colon() {
    let sql = "select :\"name\", :é";
    assert!(find_parameters(sql).is_empty());
    assert_eq!(bind_parameters(sql, &[text(":name", "x")]), sql);
}
