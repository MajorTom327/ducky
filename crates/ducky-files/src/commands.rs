use std::path::{Path, PathBuf};

use ducky_core::RestoreOptions;

use crate::FilesError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: PathBuf,
    pub arguments: Vec<String>,
    pub environment: Vec<(String, String)>,
}

struct SplitUrl {
    url_without_password: String,
    environment: Vec<(String, String)>,
}

fn split_password(database_url: &str) -> Result<SplitUrl, FilesError> {
    let mut parsed_url =
        url::Url::parse(database_url).map_err(|error| FilesError::InvalidUrl(error.to_string()))?;
    let mut environment = Vec::new();
    if let Some(encoded_password) = parsed_url.password() {
        let password = percent_encoding::percent_decode_str(encoded_password)
            .decode_utf8()
            .map_err(|error| FilesError::InvalidUrl(error.to_string()))?
            .into_owned();
        environment.push(("PGPASSWORD".to_owned(), password));
        parsed_url
            .set_password(None)
            .map_err(|()| FilesError::InvalidUrl("cannot remove the password".to_owned()))?;
    }
    Ok(SplitUrl {
        url_without_password: parsed_url.to_string(),
        environment,
    })
}

pub fn dump_command(
    pg_dump: &Path,
    database_url: &str,
    output_file: &Path,
) -> Result<CommandSpec, FilesError> {
    let split_url = split_password(database_url)?;
    Ok(CommandSpec {
        program: pg_dump.to_path_buf(),
        arguments: vec![
            "--format=custom".to_owned(),
            "--verbose".to_owned(),
            format!("--file={}", output_file.display()),
            format!("--dbname={}", split_url.url_without_password),
        ],
        environment: split_url.environment,
    })
}

pub fn restore_command(
    pg_restore: &Path,
    database_url: &str,
    input_file: &Path,
    options: &RestoreOptions,
) -> Result<CommandSpec, FilesError> {
    let split_url = split_password(database_url)?;
    let mut arguments = vec![
        "--verbose".to_owned(),
        format!("--dbname={}", split_url.url_without_password),
    ];
    if options.clean_before_restore {
        arguments.push("--clean".to_owned());
        arguments.push("--if-exists".to_owned());
    }
    if options.skip_ownership {
        arguments.push("--no-owner".to_owned());
    }
    arguments.push(input_file.display().to_string());
    Ok(CommandSpec {
        program: pg_restore.to_path_buf(),
        arguments,
        environment: split_url.environment,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const URL: &str = "postgres://ducky:p%40ss@localhost:5440/shop";

    #[test]
    fn dump_arguments_hide_password() {
        let spec = dump_command(Path::new("/bin/pg_dump"), URL, Path::new("/d/x.dump")).unwrap();
        assert_eq!(spec.program, PathBuf::from("/bin/pg_dump"));
        assert_eq!(
            spec.arguments,
            vec![
                "--format=custom",
                "--verbose",
                "--file=/d/x.dump",
                "--dbname=postgres://ducky@localhost:5440/shop",
            ]
        );
        assert_eq!(
            spec.environment,
            vec![("PGPASSWORD".to_owned(), "p@ss".to_owned())]
        );
        assert!(
            spec.arguments
                .iter()
                .all(|argument| !argument.contains("p@ss") && !argument.contains("p%40ss"))
        );
    }

    #[test]
    fn restore_arguments_follow_options() {
        let options = RestoreOptions {
            clean_before_restore: true,
            skip_ownership: true,
        };
        let spec = restore_command(
            Path::new("pg_restore"),
            URL,
            Path::new("/d/x.dump"),
            &options,
        )
        .unwrap();
        assert_eq!(
            spec.arguments,
            vec![
                "--verbose",
                "--dbname=postgres://ducky@localhost:5440/shop",
                "--clean",
                "--if-exists",
                "--no-owner",
                "/d/x.dump",
            ]
        );
        assert_eq!(
            spec.environment,
            vec![("PGPASSWORD".to_owned(), "p@ss".to_owned())]
        );
        let plain = restore_command(
            Path::new("pg_restore"),
            URL,
            Path::new("/d/x.dump"),
            &RestoreOptions::default(),
        )
        .unwrap();
        assert_eq!(plain.arguments.len(), 3);
    }

    #[test]
    fn url_without_password_has_no_environment() {
        let spec = dump_command(
            Path::new("pg_dump"),
            "postgres://localhost/db",
            Path::new("f"),
        )
        .unwrap();
        assert!(spec.environment.is_empty());
    }

    #[test]
    fn invalid_url_is_rejected() {
        assert!(matches!(
            dump_command(Path::new("pg_dump"), "not a url", Path::new("f")),
            Err(FilesError::InvalidUrl(_))
        ));
    }
}
