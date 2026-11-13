mod support;

use std::time::{SystemTime, UNIX_EPOCH};

use crossterm::event::KeyCode;
use ducky_app::{QueryView, RunState, TabContent, TableView};
use ducky_core::{Profile, ProfileKind, TransactionMode};
use ducky_postgres::PgSession;

use support::Harness;

fn active_query(harness: &Harness) -> &QueryView {
    match harness.workspace().active_tab().map(|tab| &tab.content) {
        Some(TabContent::Query(query)) => query,
        other => panic!("a query tab is not active: {other:?}"),
    }
}

fn active_table(harness: &Harness) -> &TableView {
    match harness.workspace().active_tab().map(|tab| &tab.content) {
        Some(TabContent::Table(table)) => table,
        other => panic!("a table tab is not active: {other:?}"),
    }
}

fn active_rows(harness: &Harness) -> Vec<Vec<Option<String>>> {
    active_table(harness)
        .page
        .ready()
        .expect("the page is loaded")
        .result_set
        .rows
        .clone()
}

const PROFILE_LABEL: &str = "E2E local";
const STAGING_PROFILE_LABEL: &str = "E2E staging";

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

    let mut harness = Harness::new(vec![
        Profile {
            label: PROFILE_LABEL.to_owned(),
            url: database_url.clone(),
            kind: ProfileKind::Local,
        },
        Profile {
            label: STAGING_PROFILE_LABEL.to_owned(),
            url: database_url.clone(),
            kind: ProfileKind::Staging,
        },
    ]);

    harness.press_code(KeyCode::Enter);
    harness.settle().await;
    let screen = harness.screen();
    assert!(screen.contains(PROFILE_LABEL), "{screen}");
    assert!(screen.contains("Ctrl+p"), "{screen}");

    harness.press_ctrl('p');
    harness.type_text(&format!("{schema_name}.people"));
    harness.press_code(KeyCode::Enter);
    harness.settle().await;
    assert_eq!(harness.workspace().schema_name, schema_name);
    let screen = harness.screen();
    assert!(screen.contains("people"), "{screen}");
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

    harness.press_ctrl('n');
    harness.screen();
    harness.press_char('i');
    harness.type_text(&format!(
        "insert into {schema_name}.people(name) values ('zed'); select count(*) from {schema_name}.people;"
    ));
    harness.press_code(KeyCode::F(5));
    harness.settle().await;
    match &active_query(&harness).run {
        RunState::Finished(report) => {
            assert_eq!(report.failure, None);
            assert_eq!(report.results.len(), 2);
        }
        other => panic!("script did not finish: {other:?}"),
    }
    let screen = harness.screen();
    assert!(screen.contains("151"), "{screen}");
    assert!(screen.contains("Query 1"), "{screen}");

    harness.press_code(KeyCode::Esc);
    harness.press_code(KeyCode::Tab);
    harness.press_char('1');
    assert_eq!(harness.workspace().active_tab_index, 0);
    harness.press_char('L');
    assert_eq!(harness.workspace().active_tab_index, 1);

    harness.press_ctrl('t');
    assert_eq!(
        active_query(&harness).transaction_mode,
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
    let screen = harness.screen();
    assert!(screen.contains("e2e.sql"), "{screen}");
    assert_eq!(
        active_query(&harness).ran_on_profile_label.as_deref(),
        Some(PROFILE_LABEL)
    );

    let query_tab_id = harness.workspace().active_tab().expect("query tab").id;
    harness.press_ctrl('p');
    harness.type_text("switch");
    let switch_item_index = harness
        .workspace()
        .palette_items(&harness.state.profiles, "switch")
        .iter()
        .position(|item| item.label == format!("Switch to {STAGING_PROFILE_LABEL}"))
        .expect("the staging profile is offered");
    for _ in 0..switch_item_index {
        harness.press_code(KeyCode::Down);
    }
    harness.press_code(KeyCode::Enter);
    harness.settle().await;
    assert_eq!(harness.workspace().profile.label, STAGING_PROFILE_LABEL);
    let screen = harness.screen();
    let header = screen.lines().next().unwrap_or_default();
    assert!(header.contains(STAGING_PROFILE_LABEL), "{screen}");
    assert_eq!(harness.workspace().tabs.len(), 2);
    assert_eq!(
        harness.workspace().active_tab().map(|tab| tab.id),
        Some(query_tab_id)
    );
    assert_eq!(
        harness.tui.editor(query_tab_id).text(),
        "select 1; select nope;"
    );

    harness.press_code(KeyCode::F(5));
    harness.settle().await;
    assert_eq!(
        active_query(&harness).ran_on_profile_label.as_deref(),
        Some(STAGING_PROFILE_LABEL)
    );
    let screen = harness.screen();
    assert!(
        screen.contains(&format!("· on {STAGING_PROFILE_LABEL} ")),
        "{screen}"
    );

    harness.press_ctrl('w');
    assert_eq!(harness.workspace().tabs.len(), 1);

    run_setup(
        &setup_session,
        vec![format!("DROP SCHEMA {schema_name} CASCADE")],
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs DUCKY_TEST_DATABASE_URL"]
async fn follow_foreign_keys_between_tabs() {
    let database_url =
        std::env::var("DUCKY_TEST_DATABASE_URL").expect("DUCKY_TEST_DATABASE_URL is set");
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let schema_name = format!("ducky_fk_{nanos}");
    let setup_session = PgSession::connect(&database_url)
        .await
        .expect("connects to the test database");
    run_setup(
        &setup_session,
        vec![
            format!("CREATE SCHEMA {schema_name}"),
            format!("CREATE TABLE {schema_name}.customers (id serial PRIMARY KEY, email text NOT NULL)"),
            format!(
                "CREATE TABLE {schema_name}.orders (id serial PRIMARY KEY, customer_id integer NOT NULL REFERENCES {schema_name}.customers (id), status text)"
            ),
            format!(
                "INSERT INTO {schema_name}.customers (email) SELECT 'c' || g || '@example.com' FROM generate_series(1, 3) AS g"
            ),
            format!(
                "INSERT INTO {schema_name}.orders (customer_id, status) VALUES (2, 'paid'), (2, 'pending'), (3, 'paid')"
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

    harness.press_ctrl('p');
    harness.type_text(&format!("{schema_name}.orders"));
    harness.press_code(KeyCode::Enter);
    harness.settle().await;
    assert_eq!(active_table(&harness).relation.name, "orders");
    let screen = harness.screen();
    assert!(screen.contains("customer_id →"), "{screen}");

    harness.press_char('l');
    let customer_id = active_rows(&harness)[0][1]
        .clone()
        .expect("customer_id is set");
    harness.type_text("gd");
    harness.settle().await;
    let customers_filter = format!("\"id\" = '{customer_id}'");
    assert_eq!(active_table(&harness).relation.name, "customers");
    assert_eq!(
        active_table(&harness).request.filter.as_deref(),
        Some(customers_filter.as_str())
    );
    assert_eq!(active_rows(&harness).len(), 1);
    let screen = harness.screen();
    assert!(
        screen.contains(&format!("where {customers_filter}")),
        "{screen}"
    );

    harness.type_text("gr");
    harness.settle().await;
    let orders_filter = format!("\"customer_id\" = '{customer_id}'");
    assert_eq!(active_table(&harness).relation.name, "orders");
    assert_eq!(
        active_table(&harness).request.filter.as_deref(),
        Some(orders_filter.as_str())
    );
    let referencing_rows = active_rows(&harness);
    assert!(!referencing_rows.is_empty());
    assert!(
        referencing_rows
            .iter()
            .all(|row| row[1].as_deref() == Some(customer_id.as_str()))
    );
    assert_eq!(harness.workspace().tabs.len(), 3);

    run_setup(
        &setup_session,
        vec![format!("DROP SCHEMA {schema_name} CASCADE")],
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs DUCKY_TEST_DATABASE_URL"]
async fn edit_selected_rows_opens_an_update_preset() {
    let database_url =
        std::env::var("DUCKY_TEST_DATABASE_URL").expect("DUCKY_TEST_DATABASE_URL is set");
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let schema_name = format!("ducky_edit_{nanos}");
    let setup_session = PgSession::connect(&database_url)
        .await
        .expect("connects to the test database");
    run_setup(
        &setup_session,
        vec![
            format!("CREATE SCHEMA {schema_name}"),
            format!("CREATE TABLE {schema_name}.orders (id serial PRIMARY KEY, customer_id integer, status text)"),
            format!(
                "INSERT INTO {schema_name}.orders (customer_id, status) VALUES (1, 'paid'), (2, 'pending'), (3, 'paid')"
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

    harness.press_ctrl('p');
    harness.type_text(&format!("{schema_name}.orders"));
    harness.press_code(KeyCode::Enter);
    harness.settle().await;
    assert_eq!(active_table(&harness).relation.name, "orders");
    harness.type_text("ll");
    let rows = active_rows(&harness);
    let first_id = rows[0][0].clone().expect("id is set");
    let second_id = rows[1][0].clone().expect("id is set");
    harness.type_text("vv");
    assert_eq!(active_table(&harness).selected_row_indexes(), vec![0, 1]);
    let screen = harness.screen();
    assert!(screen.contains("2 selected"), "{screen}");

    harness.press_char('e');
    let tab_title = harness
        .workspace()
        .active_tab()
        .expect("a tab is active")
        .title();
    assert!(tab_title.starts_with("Query "), "{tab_title}");
    let text = active_query(&harness).document.text.clone();
    let relation_line = format!("UPDATE \"{schema_name}\".\"orders\"");
    assert!(text.starts_with(&relation_line), "{text}");
    assert!(text.contains("\"status\" = :status"), "{text}");
    assert!(
        text.contains(&format!("WHERE \"id\" IN ('{first_id}', '{second_id}')")),
        "{text}"
    );
    let screen = harness.screen();
    assert!(screen.contains(&relation_line), "{screen}");

    run_setup(
        &setup_session,
        vec![format!("DROP SCHEMA {schema_name} CASCADE")],
    )
    .await;
}
