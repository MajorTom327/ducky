use std::time::Duration;

use ducky_app::{Command, Msg, SaveFailure};
use ducky_core::StoragePaths;
use ducky_runtime::Runtime;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

async fn next_message(receiver: &mut UnboundedReceiver<Msg>) -> Msg {
    tokio::time::timeout(Duration::from_secs(10), receiver.recv())
        .await
        .expect("a message arrives in time")
        .expect("the channel stays open")
}

fn runtime_in(directory: &tempfile::TempDir) -> (Runtime, UnboundedReceiver<Msg>) {
    let (sender, receiver) = unbounded_channel();
    let storage = StoragePaths {
        sql_dir: directory.path().join("sql"),
        dump_dir: directory.path().join("dumps"),
    };
    (Runtime::new(storage, sender), receiver)
}

#[tokio::test]
async fn save_list_and_open_round_trip() {
    let directory = tempfile::tempdir().expect("tempdir");
    let (mut runtime, mut receiver) = runtime_in(&directory);

    runtime.dispatch(Command::SaveFile {
        name: "report".to_owned(),
        content: "select 1;".to_owned(),
        overwrite: false,
    });
    match next_message(&mut receiver).await {
        Msg::FileSaved { name, result } => {
            assert_eq!(name, "report.sql");
            assert_eq!(result.expect("saved").name, "report.sql");
        }
        other => panic!("unexpected message {other:?}"),
    }

    runtime.dispatch(Command::ListSqlFiles);
    match next_message(&mut receiver).await {
        Msg::SqlFilesListed(Ok(entries)) => {
            let names: Vec<&str> = entries.iter().map(|entry| entry.name.as_str()).collect();
            assert_eq!(names, vec!["report.sql"]);
        }
        other => panic!("unexpected message {other:?}"),
    }

    runtime.dispatch(Command::OpenFile {
        name: "report.sql".to_owned(),
        run_after_open: true,
    });
    assert_eq!(
        next_message(&mut receiver).await,
        Msg::FileOpened {
            name: "report.sql".to_owned(),
            run_after_open: true,
            result: Ok("select 1;".to_owned()),
        }
    );

    runtime.dispatch(Command::LoadFilePreview {
        name: "report.sql".to_owned(),
    });
    assert_eq!(
        next_message(&mut receiver).await,
        Msg::FilePreviewLoaded {
            name: "report.sql".to_owned(),
            result: Ok("select 1;".to_owned()),
        }
    );
}

#[tokio::test]
async fn saving_twice_without_overwrite_reports_already_exists() {
    let directory = tempfile::tempdir().expect("tempdir");
    let (mut runtime, mut receiver) = runtime_in(&directory);
    let save = |overwrite: bool| Command::SaveFile {
        name: "twice.sql".to_owned(),
        content: "select 2;".to_owned(),
        overwrite,
    };

    runtime.dispatch(save(false));
    assert!(matches!(
        next_message(&mut receiver).await,
        Msg::FileSaved { result: Ok(_), .. }
    ));

    runtime.dispatch(save(false));
    assert_eq!(
        next_message(&mut receiver).await,
        Msg::FileSaved {
            name: "twice.sql".to_owned(),
            result: Err(SaveFailure::AlreadyExists),
        }
    );

    runtime.dispatch(save(true));
    assert!(matches!(
        next_message(&mut receiver).await,
        Msg::FileSaved { result: Ok(_), .. }
    ));
}

#[tokio::test]
async fn missing_directories_list_empty() {
    let directory = tempfile::tempdir().expect("tempdir");
    let (mut runtime, mut receiver) = runtime_in(&directory);

    runtime.dispatch(Command::ListSqlFiles);
    assert_eq!(
        next_message(&mut receiver).await,
        Msg::SqlFilesListed(Ok(Vec::new()))
    );
    runtime.dispatch(Command::ListDumps);
    assert_eq!(
        next_message(&mut receiver).await,
        Msg::DumpsListed(Ok(Vec::new()))
    );
}

#[tokio::test]
async fn database_commands_without_connection_answer_not_connected() {
    let directory = tempfile::tempdir().expect("tempdir");
    let (mut runtime, mut receiver) = runtime_in(&directory);

    runtime.dispatch(Command::LoadCatalog);
    assert_eq!(
        next_message(&mut receiver).await,
        Msg::CatalogLoaded(Err("Not connected".to_owned()))
    );

    runtime.dispatch(Command::RunScript {
        tab_id: 3,
        sql: "select 1".to_owned(),
        mode: ducky_core::TransactionMode::Transaction,
    });
    match next_message(&mut receiver).await {
        Msg::ScriptFinished { tab_id, report } => {
            assert_eq!(tab_id, 3);
            let failure = report.failure.expect("a failure");
            assert_eq!(failure.statement_index, 0);
            assert_eq!(failure.message, "Not connected");
            assert!(report.results.is_empty());
        }
        other => panic!("unexpected message {other:?}"),
    }

    let request = ducky_core::PageRequest {
        relation: ducky_core::RelationRef {
            schema: "public".to_owned(),
            name: "customers".to_owned(),
        },
        page_index: 0,
        page_size: 100,
        sort: None,
        filter: None,
    };
    runtime.dispatch(Command::LoadPage(request.clone()));
    assert_eq!(
        next_message(&mut receiver).await,
        Msg::PageLoaded {
            request: request.clone(),
            result: Err("Not connected".to_owned()),
        }
    );

    runtime.dispatch(Command::LoadDetails(request.relation.clone()));
    assert_eq!(
        next_message(&mut receiver).await,
        Msg::DetailsLoaded {
            relation: request.relation,
            result: Err("Not connected".to_owned()),
        }
    );

    runtime.dispatch(Command::CancelScript);
    assert_eq!(
        next_message(&mut receiver).await,
        Msg::Notified(ducky_app::Notice {
            level: ducky_app::NoticeLevel::Error,
            text: "Not connected".to_owned(),
        })
    );

    runtime.dispatch(Command::StartDump {
        custom_file_name: None,
    });
    assert_eq!(
        next_message(&mut receiver).await,
        Msg::JobFinished {
            job: ducky_app::JobKind::Dump,
            result: Err("Not connected".to_owned()),
        }
    );

    runtime.dispatch(Command::StartRestore {
        file_name: "backup.dump".to_owned(),
        options: ducky_core::RestoreOptions::default(),
    });
    assert_eq!(
        next_message(&mut receiver).await,
        Msg::JobFinished {
            job: ducky_app::JobKind::Restore,
            result: Err("Not connected".to_owned()),
        }
    );
}

#[tokio::test]
async fn file_errors_are_reported_as_text() {
    let directory = tempfile::tempdir().expect("tempdir");
    let (mut runtime, mut receiver) = runtime_in(&directory);

    runtime.dispatch(Command::OpenFile {
        name: "missing".to_owned(),
        run_after_open: false,
    });
    assert_eq!(
        next_message(&mut receiver).await,
        Msg::FileOpened {
            name: "missing".to_owned(),
            run_after_open: false,
            result: Err("missing.sql was not found".to_owned()),
        }
    );

    runtime.dispatch(Command::SaveFile {
        name: "../escape".to_owned(),
        content: "select 1;".to_owned(),
        overwrite: false,
    });
    assert_eq!(
        next_message(&mut receiver).await,
        Msg::FileSaved {
            name: "../escape".to_owned(),
            result: Err(SaveFailure::Other(
                "Invalid file name: ../escape".to_owned()
            )),
        }
    );
}
