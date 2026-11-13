use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc::UnboundedSender;

use crate::{CommandSpec, FilesError};

pub async fn run_tool(
    spec: CommandSpec,
    log_lines: UnboundedSender<String>,
) -> Result<(), FilesError> {
    let mut child = tokio::process::Command::new(&spec.program)
        .args(&spec.arguments)
        .envs(spec.environment.iter().map(|(key, value)| (key, value)))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;
    let mut last_line = String::new();
    if let Some(stderr) = child.stderr.take() {
        let mut lines = BufReader::new(stderr).lines();
        while let Some(line) = lines.next_line().await? {
            last_line.clone_from(&line);
            let _ = log_lines.send(line);
        }
    }
    let status = child.wait().await?;
    if status.success() {
        Ok(())
    } else {
        Err(FilesError::ToolFailed {
            exit_code: status.code(),
            last_line,
        })
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn forwards_stderr_and_reports_failure() {
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        let spec = CommandSpec {
            program: PathBuf::from("sh"),
            arguments: vec![
                "-c".to_owned(),
                "echo one >&2; echo two >&2; exit 3".to_owned(),
            ],
            environment: Vec::new(),
        };
        let result = run_tool(spec, sender).await;
        assert_eq!(
            result,
            Err(FilesError::ToolFailed {
                exit_code: Some(3),
                last_line: "two".to_owned()
            })
        );
        assert_eq!(receiver.recv().await.as_deref(), Some("one"));
        assert_eq!(receiver.recv().await.as_deref(), Some("two"));
        assert_eq!(receiver.recv().await, None);
    }

    #[tokio::test]
    async fn passes_environment_and_succeeds() {
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        let spec = CommandSpec {
            program: PathBuf::from("sh"),
            arguments: vec!["-c".to_owned(), "echo \"$PGPASSWORD\" >&2".to_owned()],
            environment: vec![("PGPASSWORD".to_owned(), "p@ss".to_owned())],
        };
        assert_eq!(run_tool(spec, sender).await, Ok(()));
        assert_eq!(receiver.recv().await.as_deref(), Some("p@ss"));
    }

    #[tokio::test]
    async fn spawn_error_is_io() {
        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
        let spec = CommandSpec {
            program: PathBuf::from("/nonexistent/ducky-tool"),
            arguments: Vec::new(),
            environment: Vec::new(),
        };
        assert!(matches!(
            run_tool(spec, sender).await,
            Err(FilesError::Io(_))
        ));
    }
}
