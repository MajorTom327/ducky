pub fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

pub fn quote_identifier_if_needed(identifier: &str) -> String {
    let mut identifier_chars = identifier.chars();
    let is_bare = identifier_chars
        .next()
        .is_some_and(|first_char| first_char.is_ascii_lowercase() || first_char == '_')
        && identifier_chars.all(|next_char| {
            next_char.is_ascii_lowercase() || next_char.is_ascii_digit() || next_char == '_'
        });
    if is_bare {
        identifier.to_owned()
    } else {
        quote_identifier(identifier)
    }
}

#[cfg(test)]
mod tests;
