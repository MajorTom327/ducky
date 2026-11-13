use std::time::Duration;

use ducky_app::{Command, Msg};
use ducky_core::{Profile, ProfileKind, StatementOutcome, StoragePaths, TransactionMode};
use ducky_runtime::Runtime;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

fn database_url() -> String {
    std::env::var("DUCKY_TEST_DATABASE_URL").expect("DUCKY_TEST_DATABASE_URL is set")
}

async fn next_message(receiver: &mut UnboundedReceiver<Msg>) -> Msg {
    tokio::time::timeout(Duration::from_secs(30), receiver.recv())
        .await
        .expect("a message arrives in time")
        .expect("the channel stays open")
}

fn connected_runtime(directory: &tempfile::TempDir) -> (Runtime, UnboundedReceiver<Msg>) {
    let (sender, receiver) = unbounded_channel();
    let storage = StoragePaths {
        sql_dir: directory.path().join("sql"),
        dump_dir: directory.path().join("dumps"),
    };
    let mut runtime = Runtime::new(storage, sender);
    runtime.dispatch(Command::Connect {
        profile_index: 0,
        profile: Profile {
            label: "Test".to_owned(),
            url: database_url(),
            kind: ProfileKind::Local,
            project: None,
            tags: Vec::new(),
        },
    });
    (runtime, receiver)
}

#[tokio::test]
#[ignore = "needs DUCKY_TEST_DATABASE_URL"]
async fn connect_then_load_catalog() {
    let directory = tempfile::tempdir().expect("tempdir");
    let (mut runtime, mut receiver) = connected_runtime(&directory);
    let expected_database = url::Url::parse(&database_url())
        .expect("valid url")
        .path()
        .trim_start_matches('/')
        .to_owned();

    match next_message(&mut receiver).await {
        Msg::Connected {
            profile_index,
            result,
        } => {
            assert_eq!(profile_index, 0);
            assert_eq!(
                result.expect("connected").session.database,
                expected_database
            );
        }
        other => panic!("unexpected message {other:?}"),
    }

    runtime.dispatch(Command::LoadCatalog);
    match next_message(&mut receiver).await {
        Msg::CatalogLoaded(Ok(catalog)) => {
            assert!(catalog.schema("public").is_some());
        }
        other => panic!("unexpected message {other:?}"),
    }
}

#[tokio::test]
#[ignore = "needs DUCKY_TEST_DATABASE_URL"]
async fn run_script_returns_one_result_per_statement() {
    let directory = tempfile::tempdir().expect("tempdir");
    let (mut runtime, mut receiver) = connected_runtime(&directory);
    assert!(matches!(
        next_message(&mut receiver).await,
        Msg::Connected { result: Ok(_), .. }
    ));

    runtime.dispatch(Command::RunScript {
        tab_id: 7,
        sql: "select 1; select 2".to_owned(),
        mode: TransactionMode::Transaction,
    });
    match next_message(&mut receiver).await {
        Msg::ScriptFinished { tab_id, report } => {
            assert_eq!(tab_id, 7);
            assert_eq!(report.failure, None);
            assert_eq!(report.results.len(), 2);
            assert!(
                report
                    .results
                    .iter()
                    .all(|result| matches!(result.outcome, StatementOutcome::Rows(_)))
            );
        }
        other => panic!("unexpected message {other:?}"),
    }
}

#[tokio::test]
async fn connect_with_unknown_env_var_fails() {
    let (sender, mut receiver) = unbounded_channel();
    let directory = tempfile::tempdir().expect("tempdir");
    let mut runtime = Runtime::new(
        StoragePaths {
            sql_dir: directory.path().join("sql"),
            dump_dir: directory.path().join("dumps"),
        },
        sender,
    );
    runtime.dispatch(Command::Connect {
        profile_index: 3,
        profile: Profile {
            label: "Broken".to_owned(),
            url: "postgres://ducky:${DUCKY_RUNTIME_TEST_UNSET_VARIABLE}@localhost/ducky".to_owned(),
            kind: ProfileKind::Prod,
            project: None,
            tags: Vec::new(),
        },
    });
    match next_message(&mut receiver).await {
        Msg::Connected {
            profile_index,
            result,
        } => {
            assert_eq!(profile_index, 3);
            assert!(
                result
                    .expect_err("connection fails")
                    .contains("DUCKY_RUNTIME_TEST_UNSET_VARIABLE")
            );
        }
        other => panic!("unexpected message {other:?}"),
    }
}
