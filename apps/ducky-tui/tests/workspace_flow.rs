mod support;

use std::time::{SystemTime, UNIX_EPOCH};

use crossterm::event::KeyCode;
use ducky_app::{RunState, SidebarItem};
use ducky_core::{Profile, ProfileKind, TransactionMode};
use ducky_postgres::PgSession;

use support::Harness;

const PROFILE_LABEL: &str = "E2E local";

async fn run_setup(session: &PgSession, statements: Vec<String>) {
    let report = session
        .run_script(&statements, TransactionMode::AutoCommit)
        .await;
    assert_eq!(report.failure, None, "setup statements failed");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs DUCKY_TEST_DATABASE_URL"]
async fn explore_query_and_save() {
    let database_url =
        std::env::var("DUCKY_TEST_DATABASE_URL").expect("DUCKY_TEST_DATABASE_URL is set");
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let schema_name = format!("ducky_e2e_{nanos}");
    let setup_session = PgSession::connect(&database_url)
        .await
        .expect("connects to the test database");
    run_setup(
        &setup_session,
        vec![
            format!("CREATE SCHEMA {schema_name}"),
            format!("CREATE TABLE {schema_name}.people (id serial PRIMARY KEY, name text NOT NULL)"),
            format!(
                "INSERT INTO {schema_name}.people (name) SELECT 'person ' || g FROM generate_series(1, 150) AS g"
            ),
        ],
    )
    .await;

    let mut harness = Harness::new(vec![Profile {
        label: PROFILE_LABEL.to_owned(),
        url: database_url.clone(),
        kind: ProfileKind::Local,
    }]);

    harness.press_code(KeyCode::Enter);
    harness.settle().await;
    let screen = harness.screen();
    assert!(screen.contains(PROFILE_LABEL), "{screen}");
    assert!(screen.contains("SQL command"), "{screen}");

    let schema_names = harness.workspace().schema_names();
    let schema_position = schema_names
        .iter()
        .position(|name| name == &schema_name)
        .expect("the test schema is in the catalog");
    harness.press_char('s');
    for _ in 0..schema_names.len() {
        harness.press_char('k');
    }
    for _ in 0..schema_position {
        harness.press_char('j');
    }
    harness.press_code(KeyCode::Enter);
    assert_eq!(harness.workspace().schema_name, schema_name);
    let screen = harness.screen();
    assert!(screen.contains("people"), "{screen}");

    let people_position = harness
        .workspace()
        .sidebar_items()
        .iter()
        .position(|item| {
            matches!(item, SidebarItem::Relation { relation, .. } if relation.name == "people")
        })
        .expect("people is listed in the sidebar");
    for _ in 0..people_position {
        harness.press_char('j');
    }
    harness.press_code(KeyCode::Enter);
    harness.settle().await;
    let screen = harness.screen();
    assert!(screen.contains("name"), "{screen}");
    assert!(screen.contains("page 1"), "{screen}");

    harness.press_char('n');
    harness.settle().await;
    let screen = harness.screen();
    assert!(screen.contains("page 2"), "{screen}");

    harness.press_char('t');
    harness.settle().await;
    let screen = harness.screen();
    assert!(screen.contains("Columns"), "{screen}");
    assert!(screen.contains("NOT NULL"), "{screen}");

    harness.press_code(KeyCode::Tab);
    harness.press_char('g');
    harness.press_code(KeyCode::Enter);
    harness.screen();
    harness.press_char('i');
    harness.type_text(&format!(
        "insert into {schema_name}.people(name) values ('zed'); select count(*) from {schema_name}.people;"
    ));
    harness.press_code(KeyCode::F(5));
    harness.settle().await;
    match &harness.workspace().query.run {
        RunState::Finished(report) => {
            assert_eq!(report.failure, None);
            assert_eq!(report.results.len(), 2);
        }
        other => panic!("script did not finish: {other:?}"),
    }
    let screen = harness.screen();
    assert!(screen.contains("151"), "{screen}");

    harness.press_ctrl('t');
    assert_eq!(
        harness.workspace().query.transaction_mode,
        TransactionMode::AutoCommit
    );
    harness.set_editor_text("select 1; select nope;");
    harness.press_code(KeyCode::F(5));
    harness.settle().await;
    let screen = harness.screen();
    assert!(screen.contains("Statement 2 failed"), "{screen}");

    harness.press_ctrl('s');
    harness.type_text("e2e");
    harness.press_code(KeyCode::Enter);
    harness.settle().await;
    let saved_path = harness.sql_dir().join("e2e.sql");
    let saved_content = std::fs::read_to_string(&saved_path).expect("e2e.sql was saved");
    assert!(
        saved_content.contains("select 1; select nope;"),
        "{saved_content}"
    );

    run_setup(
        &setup_session,
        vec![format!("DROP SCHEMA {schema_name} CASCADE")],
    )
    .await;
}
