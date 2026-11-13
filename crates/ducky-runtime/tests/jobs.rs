use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ducky_app::{Command, JobKind, Msg};
use ducky_core::{
    Profile, ProfileKind, RestoreOptions, ScriptReport, StatementOutcome, StoragePaths,
    TransactionMode,
};
use ducky_runtime::Runtime;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

struct Harness {
    runtime: Runtime,
    receiver: UnboundedReceiver<Msg>,
    log_lines: Vec<String>,
}

impl Harness {
    async fn next_message(&mut self) -> Msg {
        tokio::time::timeout(Duration::from_secs(60), self.receiver.recv())
            .await
            .expect("a message arrives in time")
            .expect("the channel stays open")
    }

    async fn connect(&mut self, database_url: &str) {
        self.runtime.dispatch(Command::Connect {
            profile_index: 0,
            profile: Profile {
                label: "Roundtrip".to_owned(),
                url: database_url.to_owned(),
                kind: ProfileKind::Local,
            },
        });
        match self.next_message().await {
            Msg::Connected { result, .. } => {
                result.expect("connected");
            }
            other => panic!("unexpected message {other:?}"),
        }
    }

    async fn run_script(&mut self, sql: &str, mode: TransactionMode) -> ScriptReport {
        self.runtime.dispatch(Command::RunScript {
            tab_id: 1,
            sql: sql.to_owned(),
            mode,
        });
        match self.next_message().await {
            Msg::ScriptFinished { report, .. } => report,
            other => panic!("unexpected message {other:?}"),
        }
    }

    async fn finish_job(&mut self, expected_job: JobKind) -> Result<String, String> {
        loop {
            match self.next_message().await {
                Msg::JobLogged { job, line } => {
                    assert_eq!(job, expected_job);
                    self.log_lines.push(line);
                }
                Msg::JobFinished { job, result } => {
                    assert_eq!(job, expected_job);
                    return result;
                }
                other => panic!("unexpected message {other:?}"),
            }
        }
    }
}

fn database_url_for(base_url: &str, database: &str) -> String {
    let mut database_url = url::Url::parse(base_url).expect("valid url");
    database_url.set_path(&format!("/{database}"));
    database_url.to_string()
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs DUCKY_TEST_DATABASE_URL and pg_dump / pg_restore"]
async fn dump_then_restore_roundtrip() {
    let base_url =
        std::env::var("DUCKY_TEST_DATABASE_URL").expect("DUCKY_TEST_DATABASE_URL is set");
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let source_database = format!("ducky_rt_src_{nanos}");
    let destination_database = format!("ducky_rt_dst_{nanos}");
    let directory = tempfile::tempdir().expect("tempdir");
    let (sender, receiver) = unbounded_channel();
    let mut harness = Harness {
        runtime: Runtime::new(
            StoragePaths {
                sql_dir: directory.path().join("sql"),
                dump_dir: directory.path().join("dumps"),
            },
            sender,
        ),
        receiver,
        log_lines: Vec::new(),
    };

    harness.connect(&base_url).await;
    let created = harness
        .run_script(
            &format!("create database {source_database}; create database {destination_database}"),
            TransactionMode::AutoCommit,
        )
        .await;
    assert_eq!(created.failure, None);

    harness
        .connect(&database_url_for(&base_url, &source_database))
        .await;
    let seeded = harness
        .run_script(
            "create table items(id integer primary key); insert into items values (1), (2), (3)",
            TransactionMode::Transaction,
        )
        .await;
    assert_eq!(seeded.failure, None);

    harness.runtime.dispatch(Command::StartDump {
        custom_file_name: Some("roundtrip".to_owned()),
    });
    let dumped = harness.finish_job(JobKind::Dump).await;
    let dump_file = directory.path().join("dumps").join("roundtrip.dump");
    assert_eq!(
        dumped,
        Ok(format!("Dump written to {}", dump_file.display()))
    );
    assert!(dump_file.is_file());
    assert!(!harness.log_lines.is_empty());

    harness.runtime.dispatch(Command::StartDump {
        custom_file_name: Some("roundtrip".to_owned()),
    });
    assert_eq!(
        harness.finish_job(JobKind::Dump).await,
        Err("roundtrip.dump already exists".to_owned())
    );

    harness
        .connect(&database_url_for(&base_url, &destination_database))
        .await;
    harness.runtime.dispatch(Command::StartRestore {
        file_name: "roundtrip.dump".to_owned(),
        options: RestoreOptions::default(),
    });
    assert_eq!(
        harness.finish_job(JobKind::Restore).await,
        Ok(format!(
            "Restored roundtrip.dump into {destination_database}"
        ))
    );

    let counted = harness
        .run_script("select count(*) from items", TransactionMode::Transaction)
        .await;
    assert_eq!(counted.failure, None);
    match &counted.results[0].outcome {
        StatementOutcome::Rows(result_set) => {
            assert_eq!(result_set.rows, vec![vec![Some("3".to_owned())]]);
        }
        other => panic!("unexpected outcome {other:?}"),
    }

    harness.connect(&base_url).await;
    let dropped = harness
        .run_script(
            &format!(
                "drop database {source_database} with (force); drop database {destination_database} with (force)"
            ),
            TransactionMode::AutoCommit,
        )
        .await;
    assert_eq!(dropped.failure, None);
}
