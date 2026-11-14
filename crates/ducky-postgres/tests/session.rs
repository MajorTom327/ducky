use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ducky_core::{
    PageRequest, RelationDetails, RelationKind, RelationRef, ScriptReport, Sort, SortDirection,
    StatementOutcome, TransactionMode,
};
use ducky_postgres::{MAX_RESULT_ROWS, PgError, PgSession};

static SCHEMA_COUNTER: AtomicUsize = AtomicUsize::new(0);

struct TestSchema {
    session: PgSession,
    name: String,
}

impl TestSchema {
    async fn create(setup_statements: &[&str]) -> TestSchema {
        let database_url =
            std::env::var("DUCKY_TEST_DATABASE_URL").expect("DUCKY_TEST_DATABASE_URL is set");
        let session = PgSession::connect(&database_url)
            .await
            .expect("connects to the test database");
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let name = format!(
            "ducky_it_{nanos}_{}",
            SCHEMA_COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let mut statements = vec![format!("CREATE SCHEMA {name}")];
        statements.extend(
            setup_statements
                .iter()
                .map(|statement| statement.replace("{schema}", &name)),
        );
        let report = session
            .run_script(&statements, TransactionMode::AutoCommit)
            .await;
        assert_eq!(report.failure, None, "setup failed");
        TestSchema { session, name }
    }

    async fn run(&self, statements: &[&str], mode: TransactionMode) -> ScriptReport {
        let statements: Vec<String> = statements
            .iter()
            .map(|statement| statement.replace("{schema}", &self.name))
            .collect();
        self.session.run_script(&statements, mode).await
    }

    async fn count_rows(&self, table: &str) -> String {
        let report = self
            .run(
                &[format!("SELECT count(*) FROM {{schema}}.{table}").as_str()],
                TransactionMode::AutoCommit,
            )
            .await;
        match &report.results[0].outcome {
            StatementOutcome::Rows(result_set) => {
                result_set.rows[0][0].clone().expect("count is not null")
            }
            StatementOutcome::Affected(_) => panic!("count returns rows"),
        }
    }

    fn relation(&self, name: &str) -> RelationRef {
        RelationRef {
            schema: self.name.clone(),
            name: name.to_owned(),
        }
    }

    async fn drop_schema(self) {
        let report = self
            .run(
                &["DROP SCHEMA {schema} CASCADE"],
                TransactionMode::AutoCommit,
            )
            .await;
        assert_eq!(report.failure, None, "schema drop failed");
    }
}

fn rows_of(report: &ScriptReport, result_index: usize) -> &ducky_core::ResultSet {
    match &report.results[result_index].outcome {
        StatementOutcome::Rows(result_set) => result_set,
        StatementOutcome::Affected(_) => panic!("expected rows"),
    }
}

#[tokio::test]
#[ignore]
async fn catalog_lists_tables_views_and_columns() {
    let schema = TestSchema::create(&[
        "CREATE TABLE {schema}.items (id serial PRIMARY KEY, name text NOT NULL, note varchar(20) DEFAULT 'none')",
        "CREATE VIEW {schema}.item_names AS SELECT name FROM {schema}.items",
    ])
    .await;
    let session_info = schema.session.info().await.expect("session info");
    assert!(!session_info.database.is_empty());
    assert!(!session_info.server_version.is_empty());
    let catalog = schema.session.load_catalog().await.expect("catalog");
    let items = catalog
        .relation(&schema.relation("items"))
        .expect("items listed");
    assert_eq!(items.kind, RelationKind::Table);
    let column_names: Vec<&str> = items
        .columns
        .iter()
        .map(|column| column.name.as_str())
        .collect();
    assert_eq!(column_names, ["id", "name", "note"]);
    let id_column = &items.columns[0];
    assert_eq!(id_column.data_type, "integer");
    assert!(id_column.is_primary_key);
    assert!(!id_column.is_nullable);
    assert!(
        id_column
            .default_expression
            .as_deref()
            .is_some_and(|default| default.starts_with("nextval("))
    );
    let name_column = &items.columns[1];
    assert_eq!(name_column.data_type, "text");
    assert!(!name_column.is_nullable);
    assert!(!name_column.is_primary_key);
    assert_eq!(name_column.default_expression, None);
    let note_column = &items.columns[2];
    assert_eq!(note_column.data_type, "character varying(20)");
    assert!(note_column.is_nullable);
    assert_eq!(
        note_column.default_expression.as_deref(),
        Some("'none'::character varying")
    );
    let item_names = catalog
        .relation(&schema.relation("item_names"))
        .expect("view listed");
    assert_eq!(item_names.kind, RelationKind::View);
    assert_eq!(item_names.columns.len(), 1);
    schema.drop_schema().await;
}

#[tokio::test]
#[ignore]
async fn catalog_lists_enum_labels_in_sort_order() {
    let schema = TestSchema::create(&[
        "CREATE TYPE {schema}.mood AS ENUM ('sad', 'happy')",
        "ALTER TYPE {schema}.mood ADD VALUE 'neutral' BEFORE 'happy'",
        "CREATE TABLE {schema}.moods (id serial PRIMARY KEY, current {schema}.mood NOT NULL)",
    ])
    .await;
    let catalog = schema.session.load_catalog().await.expect("catalog");
    let moods = catalog
        .relation(&schema.relation("moods"))
        .expect("moods listed");
    assert!(moods.columns[0].enum_labels.is_empty());
    assert_eq!(moods.columns[1].enum_labels, ["sad", "neutral", "happy"]);
    schema.drop_schema().await;
}

#[tokio::test]
#[ignore]
async fn catalog_lists_foreign_key_links_in_key_order() {
    let schema = TestSchema::create(&[
        "CREATE TABLE {schema}.customers (id serial PRIMARY KEY, email text)",
        "CREATE TABLE {schema}.orders (id serial PRIMARY KEY, customer_id integer REFERENCES {schema}.customers (id))",
        "CREATE TABLE {schema}.slots (day date, position integer, PRIMARY KEY (day, position))",
        "CREATE TABLE {schema}.bookings (id serial PRIMARY KEY, slot_position integer, slot_day date, CONSTRAINT bookings_slot_fkey FOREIGN KEY (slot_position, slot_day) REFERENCES {schema}.slots (position, day))",
    ])
    .await;
    let catalog = schema.session.load_catalog().await.expect("catalog");
    let orders = catalog
        .relation(&schema.relation("orders"))
        .expect("orders listed");
    assert_eq!(orders.foreign_keys.len(), 1);
    let customer_link = &orders.foreign_keys[0];
    assert_eq!(customer_link.columns, ["customer_id"]);
    assert_eq!(customer_link.referenced, schema.relation("customers"));
    assert_eq!(customer_link.referenced_columns, ["id"]);
    let bookings = catalog
        .relation(&schema.relation("bookings"))
        .expect("bookings listed");
    assert_eq!(bookings.foreign_keys.len(), 1);
    assert_eq!(bookings.foreign_keys[0].name, "bookings_slot_fkey");
    assert_eq!(
        bookings.foreign_keys[0].columns,
        ["slot_position", "slot_day"]
    );
    assert_eq!(
        bookings.foreign_keys[0].referenced_columns,
        ["position", "day"]
    );
    let referencing_customers = catalog.referencing(&schema.relation("customers"));
    assert_eq!(referencing_customers.len(), 1);
    assert_eq!(referencing_customers[0].0, schema.relation("orders"));
    assert!(
        catalog
            .relation(&schema.relation("customers"))
            .expect("customers listed")
            .foreign_keys
            .is_empty()
    );
    schema.drop_schema().await;
}

#[tokio::test]
#[ignore]
async fn details_return_indexes_foreign_keys_and_view_definition() {
    let schema = TestSchema::create(&[
        "CREATE TABLE {schema}.parents (id serial PRIMARY KEY, label text)",
        "CREATE TABLE {schema}.children (id serial PRIMARY KEY, parent_id integer NOT NULL REFERENCES {schema}.parents (id), rank integer CONSTRAINT children_rank_check CHECK (rank > 0))",
        "CREATE INDEX children_parent_idx ON {schema}.children (parent_id)",
        "CREATE VIEW {schema}.parent_labels AS SELECT label FROM {schema}.parents",
    ])
    .await;
    let children_details = schema
        .session
        .load_relation_details(&schema.relation("children"))
        .await
        .expect("children details");
    let parent_index = children_details
        .indexes
        .iter()
        .find(|index| index.name == "children_parent_idx")
        .expect("parent index listed");
    assert!(!parent_index.is_unique);
    assert!(!parent_index.is_primary);
    assert!(parent_index.definition.contains("(parent_id)"));
    assert!(
        children_details
            .indexes
            .iter()
            .any(|index| index.is_primary && index.is_unique)
    );
    assert_eq!(children_details.foreign_keys.len(), 1);
    assert!(
        children_details.foreign_keys[0]
            .definition
            .contains("REFERENCES")
    );
    assert_eq!(children_details.check_constraints.len(), 1);
    assert_eq!(
        children_details.check_constraints[0].name,
        "children_rank_check"
    );
    assert!(
        children_details.check_constraints[0]
            .definition
            .contains("rank > 0")
    );
    assert_eq!(children_details.view_definition, None);
    let view_details = schema
        .session
        .load_relation_details(&schema.relation("parent_labels"))
        .await
        .expect("view details");
    assert!(
        view_details
            .view_definition
            .as_deref()
            .is_some_and(|definition| definition.contains("label"))
    );
    schema.drop_schema().await;
}

#[tokio::test]
#[ignore]
async fn load_page_paginates_sorts_and_filters() {
    let schema = TestSchema::create(&[
        "CREATE TABLE {schema}.numbers (n integer NOT NULL)",
        "INSERT INTO {schema}.numbers SELECT generate_series(1, 250)",
    ])
    .await;
    let first_request = PageRequest {
        relation: schema.relation("numbers"),
        page_index: 0,
        page_size: 100,
        sort: Some(Sort {
            column: "n".to_owned(),
            direction: SortDirection::Ascending,
        }),
        filter: None,
    };
    let first_page = schema
        .session
        .load_page(&first_request)
        .await
        .expect("first page");
    assert_eq!(first_page.result_set.columns, ["n"]);
    assert_eq!(first_page.result_set.rows.len(), 100);
    assert_eq!(first_page.result_set.rows[0][0].as_deref(), Some("1"));
    assert!(first_page.has_next_page);
    let last_request = PageRequest {
        page_index: 2,
        ..first_request.clone()
    };
    let last_page = schema
        .session
        .load_page(&last_request)
        .await
        .expect("last page");
    assert_eq!(last_page.result_set.rows.len(), 50);
    assert!(!last_page.has_next_page);
    let descending_request = PageRequest {
        sort: Some(Sort {
            column: "n".to_owned(),
            direction: SortDirection::Descending,
        }),
        ..first_request.clone()
    };
    let descending_page = schema
        .session
        .load_page(&descending_request)
        .await
        .expect("descending page");
    assert_eq!(
        descending_page.result_set.rows[0][0].as_deref(),
        Some("250")
    );
    let filtered_request = PageRequest {
        filter: Some("n > 200".to_owned()),
        ..first_request
    };
    let filtered_page = schema
        .session
        .load_page(&filtered_request)
        .await
        .expect("filtered page");
    assert_eq!(filtered_page.result_set.rows[0][0].as_deref(), Some("201"));
    assert_eq!(filtered_page.result_set.rows.len(), 50);
    assert!(!filtered_page.has_next_page);
    schema.drop_schema().await;
}

const FAILING_INSERTS: [&str; 3] = [
    "INSERT INTO {schema}.entries VALUES (1)",
    "INSERT INTO {schema}.entries VALUES (2)",
    "INSERT INTO {schema}.entries VALUES (1)",
];

#[tokio::test]
#[ignore]
async fn transaction_failure_rolls_back_every_statement() {
    let schema =
        TestSchema::create(&["CREATE TABLE {schema}.entries (id integer PRIMARY KEY)"]).await;
    let report = schema
        .run(&FAILING_INSERTS, TransactionMode::Transaction)
        .await;
    let failure = report.failure.as_ref().expect("third insert fails");
    assert_eq!(failure.statement_index, 2);
    assert!(failure.message.starts_with("ERROR: "));
    assert_eq!(report.results.len(), 2);
    assert_eq!(schema.count_rows("entries").await, "0");
    schema.drop_schema().await;
}

#[tokio::test]
#[ignore]
async fn autocommit_failure_keeps_earlier_statements() {
    let schema =
        TestSchema::create(&["CREATE TABLE {schema}.entries (id integer PRIMARY KEY)"]).await;
    let report = schema
        .run(&FAILING_INSERTS, TransactionMode::AutoCommit)
        .await;
    assert_eq!(
        report
            .failure
            .as_ref()
            .map(|failure| failure.statement_index),
        Some(2)
    );
    assert_eq!(report.results[0].outcome, StatementOutcome::Affected(1));
    assert_eq!(schema.count_rows("entries").await, "2");
    schema.drop_schema().await;
}

async fn seeded_edit_schema() -> TestSchema {
    TestSchema::create(&[
        "CREATE TABLE {schema}.entries (id integer PRIMARY KEY, label text NOT NULL)",
        "INSERT INTO {schema}.entries VALUES (1, 'one'), (2, 'two')",
    ])
    .await
}

async fn label_of(schema: &TestSchema, id: i32) -> Option<String> {
    let sql = format!("SELECT label FROM {{schema}}.entries WHERE id = {id}");
    let report = schema
        .run(&[sql.as_str()], TransactionMode::AutoCommit)
        .await;
    rows_of(&report, 0).rows[0][0].clone()
}

#[tokio::test]
#[ignore]
async fn apply_edits_returns_the_updated_row_count_of_each_statement() {
    let schema = seeded_edit_schema().await;
    let sql = format!(
        "UPDATE {0}.entries SET label = 'uno' WHERE id = 1;\nUPDATE {0}.entries SET label = 'x' WHERE id = 99;",
        schema.name
    );
    let updated_row_counts = schema.session.apply_edits(&sql).await.expect("edits apply");
    assert_eq!(updated_row_counts, vec![1, 0]);
    assert_eq!(label_of(&schema, 1).await.as_deref(), Some("uno"));
    schema.drop_schema().await;
}

#[tokio::test]
#[ignore]
async fn apply_edits_rolls_back_earlier_statements_when_one_fails() {
    let schema = seeded_edit_schema().await;
    let sql = format!(
        "UPDATE {0}.entries SET label = 'uno' WHERE id = 1;\nUPDATE {0}.entries SET label = NULL WHERE id = 2;",
        schema.name
    );
    let error = schema
        .session
        .apply_edits(&sql)
        .await
        .expect_err("not-null violation");
    assert!(error.to_string().starts_with("ERROR: "), "{error}");
    assert_eq!(label_of(&schema, 1).await.as_deref(), Some("one"));
    schema.drop_schema().await;
}

#[tokio::test]
#[ignore]
async fn large_results_are_truncated() {
    let schema = TestSchema::create(&[]).await;
    let report = schema
        .run(
            &["SELECT generate_series(1, 10050)"],
            TransactionMode::AutoCommit,
        )
        .await;
    assert_eq!(report.failure, None);
    let result_set = rows_of(&report, 0);
    assert_eq!(result_set.rows.len(), MAX_RESULT_ROWS);
    assert!(result_set.is_truncated);
    schema.drop_schema().await;
}

#[tokio::test]
#[ignore]
async fn cancel_stops_a_running_script() {
    let schema = TestSchema::create(&[]).await;
    let statements = vec!["SELECT pg_sleep(30)".to_owned()];
    let (report, cancel_result) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(
            schema
                .session
                .run_script(&statements, TransactionMode::Transaction),
            async {
                tokio::time::sleep(Duration::from_millis(200)).await;
                schema.session.cancel_script().await
            }
        )
    })
    .await
    .expect("cancelled within 5 s");
    cancel_result.expect("cancel request sent");
    let failure = report.failure.expect("statement cancelled");
    assert_eq!(failure.statement_index, 0);
    assert!(failure.message.to_lowercase().contains("cancel"));
    let follow_up = schema
        .run(&["SELECT 1"], TransactionMode::Transaction)
        .await;
    assert_eq!(follow_up.failure, None);
    schema.drop_schema().await;
}

#[tokio::test]
#[ignore]
async fn empty_select_returns_column_names() {
    let schema =
        TestSchema::create(&["CREATE TABLE {schema}.things (id integer, label text)"]).await;
    let report = schema
        .run(
            &["SELECT id, label FROM {schema}.things WHERE false"],
            TransactionMode::AutoCommit,
        )
        .await;
    let result_set = rows_of(&report, 0);
    assert_eq!(result_set.columns, ["id", "label"]);
    assert!(result_set.rows.is_empty());
    assert!(!result_set.is_truncated);
    schema.drop_schema().await;
}

#[tokio::test]
#[ignore]
async fn load_page_reads_geometry_columns_as_geojson() {
    let database_url =
        std::env::var("DUCKY_TEST_DATABASE_URL").expect("DUCKY_TEST_DATABASE_URL is set");
    let probe_session = PgSession::connect(&database_url)
        .await
        .expect("connects to the test database");
    let availability = probe_session
        .run_script(
            &["SELECT count(*) FROM pg_available_extensions WHERE name = 'postgis'".to_owned()],
            TransactionMode::AutoCommit,
        )
        .await;
    if rows_of(&availability, 0).rows[0][0].as_deref() != Some("1") {
        return;
    }
    let extension_report = probe_session
        .run_script(
            &["CREATE EXTENSION IF NOT EXISTS postgis".to_owned()],
            TransactionMode::AutoCommit,
        )
        .await;
    assert_eq!(extension_report.failure, None, "postgis extension created");
    let schema = TestSchema::create(&[
        "CREATE TABLE {schema}.places (id serial PRIMARY KEY, name text NOT NULL, location geometry(Point, 4326) NOT NULL)",
        "INSERT INTO {schema}.places (name, location) VALUES ('Paris', ST_SetSRID(ST_MakePoint(2.35, 48.85), 4326))",
    ])
    .await;
    let page = schema
        .session
        .load_page(&PageRequest {
            relation: schema.relation("places"),
            page_index: 0,
            page_size: 100,
            sort: None,
            filter: None,
        })
        .await
        .expect("page loads");
    assert_eq!(page.result_set.columns, ["id", "name", "location"]);
    assert_eq!(
        page.result_set.rows,
        [vec![
            Some("1".to_owned()),
            Some("Paris".to_owned()),
            Some(r#"{"type":"Point","coordinates":[2.35,48.85]}"#.to_owned()),
        ]]
    );
    schema.drop_schema().await;
}

#[tokio::test]
async fn connect_failure_reports_the_underlying_cause() {
    let error = PgSession::connect("postgres://ducky@127.0.0.1:1/ducky?sslmode=disable")
        .await
        .expect_err("nothing listens on port 1");
    let PgError::Connect(message) = error else {
        panic!("expected a connect error, got {error:?}");
    };
    assert!(
        message.starts_with("error connecting to server: "),
        "{message}"
    );
    assert!(message.to_lowercase().contains("refused"), "{message}");
}

#[tokio::test]
#[ignore]
async fn database_errors_include_detail_and_hint() {
    let schema =
        TestSchema::create(&["CREATE TABLE {schema}.entries (id integer PRIMARY KEY)"]).await;
    let duplicate = schema
        .run(
            &[
                "INSERT INTO {schema}.entries VALUES (1)",
                "INSERT INTO {schema}.entries VALUES (1)",
            ],
            TransactionMode::AutoCommit,
        )
        .await;
    let duplicate_message = duplicate.failure.expect("duplicate key").message;
    assert!(
        duplicate_message.ends_with("\nDETAIL: Key (id)=(1) already exists."),
        "{duplicate_message}"
    );
    let hinted = schema
        .run(
            &["DO $$ BEGIN RAISE EXCEPTION 'boom' USING HINT = 'retry later'; END $$"],
            TransactionMode::AutoCommit,
        )
        .await;
    assert_eq!(
        hinted.failure.expect("raise fails").message,
        "ERROR: boom\nHINT: retry later"
    );
    schema.drop_schema().await;
}

#[tokio::test]
#[ignore]
async fn deferred_constraint_failure_is_reported_at_commit() {
    let schema = TestSchema::create(&[
        "CREATE TABLE {schema}.entries (id integer PRIMARY KEY DEFERRABLE INITIALLY DEFERRED)",
    ])
    .await;
    let report = schema
        .run(
            &[
                "INSERT INTO {schema}.entries VALUES (1)",
                "INSERT INTO {schema}.entries VALUES (1)",
            ],
            TransactionMode::Transaction,
        )
        .await;
    assert_eq!(report.results.len(), 2);
    let failure = report.failure.expect("commit fails");
    assert_eq!(failure.statement_index, 2);
    assert_eq!(failure.sql, "COMMIT");
    assert!(
        failure.message.contains("duplicate key"),
        "{}",
        failure.message
    );
    assert_eq!(schema.count_rows("entries").await, "0");
    schema.drop_schema().await;
}

#[tokio::test]
#[ignore]
async fn aborted_transaction_left_open_makes_begin_fail() {
    let schema = TestSchema::create(&[]).await;
    let left_open = schema
        .run(&["BEGIN", "SELECT 1 / 0"], TransactionMode::AutoCommit)
        .await;
    assert_eq!(
        left_open.failure.map(|failure| failure.statement_index),
        Some(1)
    );
    let report = schema
        .run(&["SELECT 1"], TransactionMode::Transaction)
        .await;
    assert!(report.results.is_empty());
    let failure = report.failure.expect("BEGIN fails");
    assert_eq!(failure.statement_index, 0);
    assert_eq!(failure.sql, "BEGIN");
    assert!(
        failure.message.contains("current transaction is aborted"),
        "{}",
        failure.message
    );
    let rolled_back = schema.run(&["ROLLBACK"], TransactionMode::AutoCommit).await;
    assert_eq!(rolled_back.failure, None);
    schema.drop_schema().await;
}

#[tokio::test]
#[ignore]
async fn lost_connection_reports_the_failed_rollback() {
    let database_url =
        std::env::var("DUCKY_TEST_DATABASE_URL").expect("DUCKY_TEST_DATABASE_URL is set");
    let session = PgSession::connect(&database_url)
        .await
        .expect("connects to the test database");
    let report = session
        .run_script(
            &["SELECT pg_terminate_backend(pg_backend_pid())".to_owned()],
            TransactionMode::Transaction,
        )
        .await;
    let failure = report.failure.expect("backend terminated");
    assert_eq!(failure.statement_index, 0);
    assert!(
        failure.message.starts_with("FATAL: terminating connection"),
        "{}",
        failure.message
    );
    assert!(
        failure
            .message
            .ends_with("\nROLLBACK failed: connection closed"),
        "{}",
        failure.message
    );
}

#[tokio::test]
#[ignore]
async fn empty_script_runs_nothing() {
    let schema = TestSchema::create(&[]).await;
    let report = schema.run(&[], TransactionMode::Transaction).await;
    assert_eq!(report.mode, TransactionMode::Transaction);
    assert!(report.results.is_empty());
    assert_eq!(report.failure, None);
    let follow_up = schema
        .run(&["SELECT 1"], TransactionMode::Transaction)
        .await;
    assert_eq!(follow_up.failure, None);
    schema.drop_schema().await;
}

#[tokio::test]
#[ignore]
async fn invalid_page_filter_returns_the_database_error() {
    let schema =
        TestSchema::create(&["CREATE TABLE {schema}.entries (id integer PRIMARY KEY)"]).await;
    let request = PageRequest {
        relation: schema.relation("entries"),
        page_index: 0,
        page_size: 10,
        sort: None,
        filter: Some("missing_column = 1".to_owned()),
    };
    let error = schema
        .session
        .load_page(&request)
        .await
        .expect_err("unknown column");
    assert_eq!(
        error,
        PgError::Query("ERROR: column \"missing_column\" does not exist".to_owned())
    );
    schema.drop_schema().await;
}

#[tokio::test]
#[ignore]
async fn details_of_a_missing_relation_are_empty() {
    let schema = TestSchema::create(&[]).await;
    let details = schema
        .session
        .load_relation_details(&schema.relation("missing"))
        .await
        .expect("details query succeeds");
    assert_eq!(
        details,
        RelationDetails {
            indexes: Vec::new(),
            foreign_keys: Vec::new(),
            check_constraints: Vec::new(),
            view_definition: None,
        }
    );
    assert_eq!(format!("{:?}", schema.session), "PgSession { .. }");
    schema.drop_schema().await;
}
