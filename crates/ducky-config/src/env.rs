use crate::ConfigError;

pub(crate) enum UrlPart<'a> {
    Literal(&'a str),
    Variable(&'a str),
}

pub(crate) fn split_url(raw_url: &str) -> Result<Vec<UrlPart<'_>>, String> {
    let mut url_parts = Vec::new();
    let mut remaining = raw_url;
    while let Some(start) = remaining.find("${") {
        url_parts.push(UrlPart::Literal(&remaining[..start]));
        let after_open = &remaining[start + 2..];
        let end = after_open
            .find('}')
            .ok_or_else(|| "unterminated `${`".to_owned())?;
        let variable = &after_open[..end];
        if !is_valid_variable_name(variable) {
            return Err(format!("invalid variable name `{variable}`"));
        }
        url_parts.push(UrlPart::Variable(variable));
        remaining = &after_open[end + 1..];
    }
    url_parts.push(UrlPart::Literal(remaining));
    Ok(url_parts)
}

fn is_valid_variable_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_alphabetic() || first == '_')
        && chars.all(|next| next.is_ascii_alphanumeric() || next == '_')
}

pub fn expand_env(
    raw_url: &str,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Result<String, ConfigError> {
    let url_parts = split_url(raw_url).map_err(|reason| ConfigError::InvalidUrl {
        label: String::new(),
        reason,
    })?;
    let mut expanded_url = String::with_capacity(raw_url.len());
    for url_part in url_parts {
        match url_part {
            UrlPart::Literal(text) => expanded_url.push_str(text),
            UrlPart::Variable(variable) => {
                let value = lookup(variable).ok_or_else(|| ConfigError::MissingEnvVar {
                    variable: variable.to_owned(),
                })?;
                expanded_url.push_str(&value);
            }
        }
    }
    Ok(expanded_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lookup(name: &str) -> Option<String> {
        match name {
            "USER_NAME" => Some("ducky".to_owned()),
            "PASSWORD" => Some("secret".to_owned()),
            _ => None,
        }
    }

    #[test]
    fn no_variables() {
        assert_eq!(
            expand_env("postgres://h/db", &lookup).unwrap(),
            "postgres://h/db"
        );
    }

    #[test]
    fn one_variable() {
        assert_eq!(
            expand_env("postgres://u:${PASSWORD}@h/db", &lookup).unwrap(),
            "postgres://u:secret@h/db"
        );
    }

    #[test]
    fn two_variables() {
        assert_eq!(
            expand_env("postgres://${USER_NAME}:${PASSWORD}@h/db", &lookup).unwrap(),
            "postgres://ducky:secret@h/db"
        );
    }

    #[test]
    fn missing_variable() {
        assert_eq!(
            expand_env("postgres://u:${NOPE}@h/db", &lookup).unwrap_err(),
            ConfigError::MissingEnvVar {
                variable: "NOPE".to_owned()
            }
        );
    }
}
