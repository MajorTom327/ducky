use super::*;

#[test]
fn quote_identifier_always_quotes() {
    assert_eq!(quote_identifier("users"), "\"users\"");
    assert_eq!(quote_identifier(""), "\"\"");
}

#[test]
fn quote_identifier_doubles_inner_quotes() {
    assert_eq!(quote_identifier("a\"b"), "\"a\"\"b\"");
}

#[test]
fn quote_identifier_if_needed_keeps_simple_names_bare() {
    assert_eq!(quote_identifier_if_needed("order_items2"), "order_items2");
    assert_eq!(quote_identifier_if_needed("_tmp"), "_tmp");
}

#[test]
fn quote_identifier_if_needed_quotes_other_names() {
    assert_eq!(quote_identifier_if_needed("Col"), "\"Col\"");
    assert_eq!(quote_identifier_if_needed("2fa"), "\"2fa\"");
    assert_eq!(quote_identifier_if_needed("with space"), "\"with space\"");
    assert_eq!(quote_identifier_if_needed(""), "\"\"");
    assert_eq!(quote_identifier_if_needed("é"), "\"é\"");
}
