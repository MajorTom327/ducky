use ducky_app::{Msg, SaveFailure};
use ducky_files::{DumpStore, FilesError, SqlStore};
use tokio::sync::mpsc::UnboundedSender;

fn spawn_file_task<F>(messages: UnboundedSender<Msg>, work: F)
where
    F: FnOnce() -> Msg + Send + 'static,
{
    tokio::task::spawn_blocking(move || {
        let _ = messages.send(work());
    });
}

fn describe(error: FilesError) -> String {
    error.to_string()
}

pub(crate) fn spawn_list_sql_files(store: SqlStore, messages: UnboundedSender<Msg>) {
    spawn_file_task(messages, move || {
        Msg::SqlFilesListed(store.list().map_err(describe))
    });
}

pub(crate) fn spawn_load_preview(store: SqlStore, messages: UnboundedSender<Msg>, name: String) {
    spawn_file_task(messages, move || {
        let result = store.preview(&name).map_err(describe);
        Msg::FilePreviewLoaded { name, result }
    });
}

pub(crate) fn spawn_open_file(
    store: SqlStore,
    messages: UnboundedSender<Msg>,
    name: String,
    run_after_open: bool,
) {
    spawn_file_task(messages, move || {
        let result = store.load(&name).map_err(describe);
        Msg::FileOpened {
            name,
            run_after_open,
            result,
        }
    });
}

pub(crate) fn spawn_save_file(
    store: SqlStore,
    messages: UnboundedSender<Msg>,
    name: String,
    content: String,
    overwrite: bool,
) {
    spawn_file_task(messages, move || {
        match store.save(&name, &content, overwrite) {
            Ok(entry) => Msg::FileSaved {
                name: entry.name.clone(),
                result: Ok(entry),
            },
            Err(FilesError::AlreadyExists(_)) => Msg::FileSaved {
                name,
                result: Err(SaveFailure::AlreadyExists),
            },
            Err(error) => Msg::FileSaved {
                name,
                result: Err(SaveFailure::Other(error.to_string())),
            },
        }
    });
}

pub(crate) fn spawn_list_dumps(store: DumpStore, messages: UnboundedSender<Msg>) {
    spawn_file_task(messages, move || {
        Msg::DumpsListed(store.list().map_err(describe))
    });
}
