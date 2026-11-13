use std::path::Path;

use ducky_app::{JobKind, Msg};
use ducky_core::{RestoreOptions, ToolStatus};
use ducky_files::{CommandSpec, DumpStore, FilesError, run_tool};
use tokio::sync::mpsc::UnboundedSender;

use crate::connection::{ActiveConnection, ActiveSlot, NOT_CONNECTED, current_connection};

pub(crate) fn spawn_dump(
    active: &ActiveSlot,
    dump_store: DumpStore,
    messages: UnboundedSender<Msg>,
    custom_file_name: Option<String>,
) {
    let connection = current_connection(active);
    tokio::spawn(async move {
        let result = match connection {
            Some(connection) => dump(&connection, &dump_store, &messages, custom_file_name).await,
            None => Err(NOT_CONNECTED.to_owned()),
        };
        let _ = messages.send(Msg::JobFinished {
            job: JobKind::Dump,
            result,
        });
    });
}

pub(crate) fn spawn_restore(
    active: &ActiveSlot,
    dump_store: DumpStore,
    messages: UnboundedSender<Msg>,
    file_name: String,
    options: RestoreOptions,
) {
    let connection = current_connection(active);
    tokio::spawn(async move {
        let result = match connection {
            Some(connection) => {
                restore(&connection, &dump_store, &messages, &file_name, options).await
            }
            None => Err(NOT_CONNECTED.to_owned()),
        };
        let _ = messages.send(Msg::JobFinished {
            job: JobKind::Restore,
            result,
        });
    });
}

async fn dump(
    connection: &ActiveConnection,
    dump_store: &DumpStore,
    messages: &UnboundedSender<Msg>,
    custom_file_name: Option<String>,
) -> Result<String, String> {
    let pg_dump = available_tool(&connection.tools.pg_dump)?;
    let file_name = custom_file_name.unwrap_or_else(|| {
        ducky_files::default_dump_name(&connection.database, chrono::Local::now())
    });
    let output_file = dump_store.path_for(&file_name).map_err(describe)?;
    if tokio::fs::try_exists(&output_file)
        .await
        .map_err(|error| error.to_string())?
    {
        return Err(format!("{} already exists", display_name(&output_file)));
    }
    if let Some(dump_dir) = output_file.parent() {
        tokio::fs::create_dir_all(dump_dir)
            .await
            .map_err(|error| error.to_string())?;
    }
    let spec = ducky_files::dump_command(pg_dump, &connection.database_url, &output_file)
        .map_err(describe)?;
    if let Err(error) = run_logged(spec, JobKind::Dump, messages).await {
        let _ = tokio::fs::remove_file(&output_file).await;
        return Err(describe(error));
    }
    Ok(format!("Dump written to {}", output_file.display()))
}

async fn restore(
    connection: &ActiveConnection,
    dump_store: &DumpStore,
    messages: &UnboundedSender<Msg>,
    file_name: &str,
    options: RestoreOptions,
) -> Result<String, String> {
    let pg_restore = available_tool(&connection.tools.pg_restore)?;
    let input_file = dump_store.path_for(file_name).map_err(describe)?;
    let spec =
        ducky_files::restore_command(pg_restore, &connection.database_url, &input_file, &options)
            .map_err(describe)?;
    run_logged(spec, JobKind::Restore, messages)
        .await
        .map_err(describe)?;
    Ok(format!("Restored {file_name} into {}", connection.database))
}

async fn run_logged(
    spec: CommandSpec,
    job: JobKind,
    messages: &UnboundedSender<Msg>,
) -> Result<(), FilesError> {
    let (log_sender, mut log_receiver) = tokio::sync::mpsc::unbounded_channel();
    let log_messages = messages.clone();
    let forwarder = tokio::spawn(async move {
        while let Some(line) = log_receiver.recv().await {
            let _ = log_messages.send(Msg::JobLogged { job, line });
        }
    });
    let result = run_tool(spec, log_sender).await;
    let _ = forwarder.await;
    result
}

fn available_tool(status: &ToolStatus) -> Result<&Path, String> {
    match status {
        ToolStatus::Available(path) => Ok(path.as_path()),
        ToolStatus::Missing(reason) => Err(reason.clone()),
    }
}

fn display_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

fn describe(error: FilesError) -> String {
    error.to_string()
}
