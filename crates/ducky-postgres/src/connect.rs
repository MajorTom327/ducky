use std::str::FromStr;
use std::time::Duration;

use tokio_postgres::config::SslMode;

use crate::error::PgError;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone)]
pub(crate) struct ConnectTarget {
    pub config: tokio_postgres::Config,
    pub verifies_certificates: bool,
}

pub(crate) fn parse_database_url(database_url: &str) -> Result<ConnectTarget, PgError> {
    let mut parsed_url =
        url::Url::parse(database_url).map_err(|error| PgError::InvalidUrl(error.to_string()))?;
    let requested_ssl_mode = parsed_url
        .query_pairs()
        .filter(|(key, _)| key == "sslmode")
        .map(|(_, value)| value.into_owned())
        .last();
    let (ssl_mode, verifies_certificates) = map_ssl_mode(requested_ssl_mode.as_deref())?;
    let remaining_query = parsed_url.query().map(|query| {
        query
            .split('&')
            .filter(|pair| !pair.is_empty() && pair.split('=').next() != Some("sslmode"))
            .collect::<Vec<_>>()
            .join("&")
    });
    match remaining_query.as_deref() {
        Some(query) if !query.is_empty() => parsed_url.set_query(Some(query)),
        _ => parsed_url.set_query(None),
    }
    let mut config = tokio_postgres::Config::from_str(parsed_url.as_str())
        .map_err(|error| PgError::InvalidUrl(crate::error::describe(&error)))?;
    config
        .ssl_mode(ssl_mode)
        .application_name("ducky")
        .connect_timeout(CONNECT_TIMEOUT);
    Ok(ConnectTarget {
        config,
        verifies_certificates,
    })
}

fn map_ssl_mode(requested_ssl_mode: Option<&str>) -> Result<(SslMode, bool), PgError> {
    match requested_ssl_mode {
        Some("disable") => Ok((SslMode::Disable, false)),
        None | Some("allow") | Some("prefer") => Ok((SslMode::Prefer, false)),
        Some("require") => Ok((SslMode::Require, false)),
        Some("verify-ca") | Some("verify-full") => Ok((SslMode::Require, true)),
        Some(unknown_mode) => Err(PgError::InvalidUrl(format!(
            "unsupported sslmode `{unknown_mode}` (expected disable, allow, prefer, require, verify-ca or verify-full)"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(database_url: &str) -> ConnectTarget {
        parse_database_url(database_url).expect("valid url")
    }

    #[test]
    fn absent_sslmode_prefers_without_verification() {
        let parsed_target = target("postgres://alice:secret@localhost:5440/ducky");
        assert_eq!(parsed_target.config.get_ssl_mode(), SslMode::Prefer);
        assert!(!parsed_target.verifies_certificates);
        assert_eq!(parsed_target.config.get_user(), Some("alice"));
        assert_eq!(parsed_target.config.get_dbname(), Some("ducky"));
        assert_eq!(parsed_target.config.get_application_name(), Some("ducky"));
        assert_eq!(
            parsed_target.config.get_connect_timeout(),
            Some(&Duration::from_secs(10))
        );
    }

    #[test]
    fn sslmode_values_map_to_driver_modes() {
        let expected_mappings = [
            ("disable", SslMode::Disable, false),
            ("allow", SslMode::Prefer, false),
            ("prefer", SslMode::Prefer, false),
            ("require", SslMode::Require, false),
            ("verify-ca", SslMode::Require, true),
            ("verify-full", SslMode::Require, true),
        ];
        for (requested_mode, expected_mode, expected_verification) in expected_mappings {
            let parsed_target = target(&format!(
                "postgresql://localhost/ducky?sslmode={requested_mode}"
            ));
            assert_eq!(
                parsed_target.config.get_ssl_mode(),
                expected_mode,
                "{requested_mode}"
            );
            assert_eq!(
                parsed_target.verifies_certificates, expected_verification,
                "{requested_mode}"
            );
        }
    }

    #[test]
    fn other_query_parameters_are_kept() {
        let parsed_target = target("postgres://localhost/ducky?sslmode=verify-full&user=bob");
        assert_eq!(parsed_target.config.get_user(), Some("bob"));
        assert_eq!(parsed_target.config.get_ssl_mode(), SslMode::Require);
    }

    #[test]
    fn unknown_sslmode_is_invalid_url() {
        let parse_result = parse_database_url("postgres://localhost/ducky?sslmode=sometimes");
        assert!(
            matches!(parse_result, Err(PgError::InvalidUrl(message)) if message.contains("sometimes"))
        );
    }

    #[test]
    fn unparsable_url_is_invalid_url() {
        assert!(matches!(
            parse_database_url("not a url"),
            Err(PgError::InvalidUrl(_))
        ));
    }
}
